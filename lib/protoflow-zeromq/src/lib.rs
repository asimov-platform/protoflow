// This is free and unencumbered software released into the public domain.

#![no_std]
#![deny(unsafe_code)]

#[doc(hidden)]
pub use protoflow_core::prelude;

#[path = "protoflow.zmq.rs"]
mod protoflow_zmq;

extern crate std;

use protoflow_core::{
    prelude::{vec, Arc, BTreeMap, Bytes, String, ToString, Vec},
    InputPortID, OutputPortID, PortError, PortResult, PortState, Transport,
};

use core::fmt::Error;
use std::{format, write};
use tokio::sync::{
    mpsc::{channel as sync_channel, error::TryRecvError, Receiver, Sender},
    Mutex, RwLock,
};
use zeromq::{util::PeerIdentity, Socket, SocketOptions, SocketRecv, SocketSend, ZmqMessage};

#[cfg(feature = "tracing")]
use tracing::{trace, trace_span};

const DEFAULT_PUB_SOCKET: &str = "tcp://127.0.0.1:10000";
const DEFAULT_SUB_SOCKET: &str = "tcp://127.0.0.1:10001";

pub struct ZmqTransport {
    tokio: tokio::runtime::Handle,

    pub_queue: Sender<ZmqTransportEvent>,
    sub_queue: Sender<ZmqSubscriptionRequest>,

    outputs: Arc<RwLock<BTreeMap<OutputPortID, RwLock<ZmqOutputPortState>>>>,
    inputs: Arc<RwLock<BTreeMap<InputPortID, RwLock<ZmqInputPortState>>>>,
}

#[derive(Debug, Clone)]
enum ZmqOutputPortState {
    Open(
        Sender<(InputPortID, Sender<Result<(), PortError>>)>,
        Sender<ZmqTransportEvent>,
    ),
    Connected(
        // channel for public send, contained channel is for the ack back
        Sender<(ZmqOutputPortRequest, Sender<Result<(), PortError>>)>,
        // internal channel for events
        Sender<ZmqTransportEvent>,
        // id of the connected input port
        InputPortID,
    ),
    Closed,
}

#[derive(Debug, Clone)]
enum ZmqOutputPortRequest {
    Close,
    Send(Bytes),
}

impl ZmqOutputPortState {
    fn state(&self) -> PortState {
        use ZmqOutputPortState::*;
        match self {
            Open(_, _) => PortState::Open,
            Connected(_, _, _) => PortState::Connected,
            Closed => PortState::Closed,
        }
    }
}

#[derive(Debug, Clone)]
enum ZmqInputPortState {
    Open(Sender<ZmqTransportEvent>),
    Connected(
        // channel for requests from public close
        Sender<(ZmqInputPortRequest, Sender<Result<(), PortError>>)>,
        // channel for the public recv
        Sender<ZmqInputPortEvent>,
        Arc<Mutex<Receiver<ZmqInputPortEvent>>>,
        // internal  channel for events
        Sender<ZmqTransportEvent>,
        // vec of the connected port ids
        Vec<OutputPortID>,
    ),
    Closed,
}

#[derive(Debug, Clone)]
enum ZmqInputPortRequest {
    Close,
}

impl ZmqInputPortState {
    fn state(&self) -> PortState {
        use ZmqInputPortState::*;
        match self {
            Open(_) => PortState::Open,
            Connected(_, _, _, _, _) => PortState::Connected,
            Closed => PortState::Closed,
        }
    }
}

type SequenceID = u64;

/// ZmqTransportEvent represents the data that goes over the wire from one port to another.
#[derive(Clone, Debug)]
enum ZmqTransportEvent {
    Connect(OutputPortID, InputPortID),
    AckConnection(OutputPortID, InputPortID),
    Message(OutputPortID, InputPortID, SequenceID, Bytes),
    AckMessage(OutputPortID, InputPortID, SequenceID),
    CloseOutput(OutputPortID, InputPortID),
    CloseInput(InputPortID),
}

impl ZmqTransportEvent {
    fn write_topic<W: std::io::Write + ?Sized>(&self, f: &mut W) -> Result<(), std::io::Error> {
        use ZmqTransportEvent::*;
        match self {
            Connect(o, i) => write!(f, "{}:conn:{}", i, o),
            AckConnection(o, i) => write!(f, "{}:ackConn:{}", i, o),
            Message(o, i, seq, _) => write!(f, "{}:msg:{}:{}", i, o, seq),
            AckMessage(o, i, seq) => write!(f, "{}:ackMsg:{}:{}", i, o, seq),
            CloseOutput(o, i) => write!(f, "{}:closeOut:{}", i, o),
            CloseInput(i) => write!(f, "{}:closeIn", i),
        }
    }
}

fn input_topics(id: InputPortID) -> Vec<String> {
    vec![
        format!("{}:conn", id),
        format!("{}:msg", id),
        format!("{}:closeOut", id),
    ]
}

fn output_topics(source: OutputPortID, target: InputPortID) -> Vec<String> {
    vec![
        format!("{}:ackConn:{}", target, source),
        format!("{}:ackMsg:{}:", target, source),
        format!("{}:closeIn", target),
    ]
}

impl From<ZmqTransportEvent> for ZmqMessage {
    fn from(value: ZmqTransportEvent) -> Self {
        let mut topic = Vec::new();
        value.write_topic(&mut topic).unwrap();

        // first frame of the message is the topic
        let mut msg = ZmqMessage::from(topic);

        fn map_id<T>(id: T) -> i64
        where
            isize: From<T>,
        {
            isize::from(id) as i64
        }

        // second frame of the message is the payload
        use prost::Message;
        use protoflow_zmq::{event::Payload, Event};
        use ZmqTransportEvent::*;
        let payload = match value {
            Connect(output, input) => Payload::Connect(protoflow_zmq::Connect {
                output: map_id(output),
                input: map_id(input),
            }),
            AckConnection(output, input) => Payload::AckConnection(protoflow_zmq::AckConnection {
                output: map_id(output),
                input: map_id(input),
            }),
            Message(output, input, sequence, message) => Payload::Message(protoflow_zmq::Message {
                output: map_id(output),
                input: map_id(input),
                sequence,
                message: message.to_vec(),
            }),
            AckMessage(output, input, sequence) => Payload::AckMessage(protoflow_zmq::AckMessage {
                output: map_id(output),
                input: map_id(input),
                sequence,
            }),
            CloseOutput(output, input) => Payload::CloseOutput(protoflow_zmq::CloseOutput {
                output: map_id(output),
                input: map_id(input),
            }),
            CloseInput(input) => Payload::CloseInput(protoflow_zmq::CloseInput {
                input: map_id(input),
            }),
        };

        let bytes = Event {
            payload: Some(payload),
        }
        .encode_to_vec();
        msg.push_back(bytes.into());

        msg
    }
}

impl TryFrom<ZmqMessage> for ZmqTransportEvent {
    type Error = protoflow_core::DecodeError;

    fn try_from(value: ZmqMessage) -> Result<Self, Self::Error> {
        use prost::Message;
        use protoflow_core::DecodeError;
        use protoflow_zmq::{event::Payload, Event};

        fn map_id<T>(id: i64) -> Result<T, DecodeError>
        where
            T: TryFrom<isize>,
            std::borrow::Cow<'static, str>: From<<T as TryFrom<isize>>::Error>,
        {
            (id as isize).try_into().map_err(DecodeError::new)
        }

        value
            .get(1)
            .ok_or_else(|| {
                protoflow_core::DecodeError::new(
                    "message from socket contains less than two frames",
                )
            })
            .and_then(|bytes| {
                let event = Event::decode(bytes.as_ref())?;

                use ZmqTransportEvent::*;
                Ok(match event.payload {
                    None => todo!(),
                    Some(Payload::Connect(protoflow_zmq::Connect { output, input })) => {
                        Connect(map_id(output)?, map_id(input)?)
                    }

                    Some(Payload::AckConnection(protoflow_zmq::AckConnection {
                        output,
                        input,
                    })) => AckConnection(map_id(output)?, map_id(input)?),

                    Some(Payload::Message(protoflow_zmq::Message {
                        output,
                        input,
                        sequence,
                        message,
                    })) => Message(
                        map_id(output)?,
                        map_id(input)?,
                        sequence,
                        Bytes::from(message),
                    ),

                    Some(Payload::AckMessage(protoflow_zmq::AckMessage {
                        output,
                        input,
                        sequence,
                    })) => AckMessage(map_id(output)?, map_id(input)?, sequence),

                    Some(Payload::CloseOutput(protoflow_zmq::CloseOutput { output, input })) => {
                        CloseOutput(map_id(output)?, map_id(input)?)
                    }

                    Some(Payload::CloseInput(protoflow_zmq::CloseInput { input })) => {
                        CloseInput(map_id(input)?)
                    }
                })
            })
    }
}

/// ZmqInputPortEvent represents events that we receive from the background worker of the port.
#[derive(Clone, Debug)]
enum ZmqInputPortEvent {
    Message(Bytes),
    Closed,
}

impl Default for ZmqTransport {
    fn default() -> Self {
        Self::new(DEFAULT_PUB_SOCKET, DEFAULT_SUB_SOCKET)
    }
}

#[derive(Clone, Debug)]
enum ZmqSubscriptionRequest {
    Subscribe(String),
    Unsubscribe(String),
}

impl ZmqTransport {
    pub fn new(pub_url: &str, sub_url: &str) -> Self {
        let tokio = tokio::runtime::Handle::current();

        let peer_id = PeerIdentity::new();

        let psock = {
            let peer_id = peer_id.clone();
            let mut sock_opts = SocketOptions::default();
            sock_opts.peer_identity(peer_id);

            let mut psock = zeromq::PubSocket::with_options(sock_opts);
            tokio
                .block_on(psock.connect(pub_url))
                .expect("failed to connect PUB");
            psock
        };

        let ssock = {
            let mut sock_opts = SocketOptions::default();
            sock_opts.peer_identity(peer_id);

            let mut ssock = zeromq::SubSocket::with_options(sock_opts);
            tokio
                .block_on(ssock.connect(sub_url))
                .expect("failed to connect SUB");
            ssock
        };

        let outputs = Arc::new(RwLock::new(BTreeMap::default()));
        let inputs = Arc::new(RwLock::new(BTreeMap::default()));

        let (pub_queue, pub_queue_recv) = sync_channel(1);

        let (sub_queue, sub_queue_recv) = tokio::sync::mpsc::channel(1);

        let transport = Self {
            pub_queue,
            sub_queue,
            tokio,
            outputs,
            inputs,
        };

        transport.start_pub_socket_worker(psock, pub_queue_recv);
        transport.start_sub_socket_worker(ssock, sub_queue_recv);

        transport
    }

    fn start_pub_socket_worker(
        &self,
        psock: zeromq::PubSocket,
        pub_queue: Receiver<ZmqTransportEvent>,
    ) {
        let mut psock = psock;
        let mut pub_queue = pub_queue;
        tokio::task::spawn(async move {
            while let Some(event) = pub_queue.recv().await {
                #[cfg(feature = "tracing")]
                trace!(
                    target: "ZmqTransport::pub_socket",
                    ?event,
                    "sending event to socket"
                );

                psock
                    .send(event.into())
                    .await
                    .expect("zmq pub-socket worker")
            }
        });
    }

    fn start_sub_socket_worker(
        &self,
        ssock: zeromq::SubSocket,
        sub_queue: Receiver<ZmqSubscriptionRequest>,
    ) {
        let outputs = self.outputs.clone();
        let inputs = self.inputs.clone();
        let mut ssock = ssock;
        let mut sub_queue = sub_queue;
        tokio::task::spawn(async move {
            loop {
                tokio::select! {
                    Ok(msg) = ssock.recv() => {
                        #[cfg(feature = "tracing")]
                        trace!(
                            target: "ZmqTransport::sub_socket",
                            ?msg,
                            "got message from socket"
                        );

                        handle_zmq_msg(msg, &outputs, &inputs).await.unwrap()
                    },
                    Some(req) = sub_queue.recv() => {
                        #[cfg(feature = "tracing")]
                        trace!(
                            target: "ZmqTransport::sub_socket",
                            ?req,
                            "got sub update request"
                        );

                        use ZmqSubscriptionRequest::*;
                        match req {
                            Subscribe(topic) => ssock.subscribe(&topic).await.expect("zmq recv worker subscribe"),
                            Unsubscribe(topic) => ssock.unsubscribe(&topic).await.expect("zmq recv worker unsubscribe"),
                        };
                    }
                };
            }
        });
    }

    fn start_input_worker(&self, input_port_id: InputPortID) -> Result<(), PortError> {
        #[cfg(feature = "tracing")]
        let span = trace_span!("ZmqTransport::start_input_worker", ?input_port_id);

        let (to_worker_send, mut to_worker_recv) = sync_channel(1);

        let (req_send, mut req_recv) = sync_channel(1);

        {
            let mut inputs = self.tokio.block_on(self.inputs.write());
            if inputs.contains_key(&input_port_id) {
                return Ok(()); // TODO
            }
            let state = ZmqInputPortState::Open(to_worker_send.clone());
            let state = RwLock::new(state);

            #[cfg(feature = "tracing")]
            span.in_scope(|| trace!(?state, "saving new opened state"));

            inputs.insert(input_port_id, state);
        }

        {
            let mut handles = Vec::new();
            for topic in input_topics(input_port_id).into_iter() {
                #[cfg(feature = "tracing")]
                span.in_scope(|| trace!(?topic, "sending subscription request"));

                let handle = self
                    .sub_queue
                    .send(ZmqSubscriptionRequest::Subscribe(topic));
                handles.push(handle);
            }
            for handle in handles.into_iter() {
                self.tokio
                    .block_on(handle)
                    .expect("input worker send sub req");
            }
        }

        let sub_queue = self.sub_queue.clone();
        let pub_queue = self.pub_queue.clone();
        let inputs = self.inputs.clone();

        async fn handle_socket_event(
            event: ZmqTransportEvent,
            inputs: &RwLock<BTreeMap<InputPortID, RwLock<ZmqInputPortState>>>,
            req_send: &Sender<(ZmqInputPortRequest, Sender<Result<(), PortError>>)>,
            pub_queue: &Sender<ZmqTransportEvent>,
            input_port_id: InputPortID,
        ) {
            #[cfg(feature = "tracing")]
            let span = trace_span!(
                "ZmqTransport::start_input_worker::handle_socket_event",
                ?input_port_id
            );

            #[cfg(feature = "tracing")]
            span.in_scope(|| trace!(?event, "got socket event"));

            use ZmqTransportEvent::*;
            match event {
                Connect(output_port_id, input_port_id) => {
                    let inputs = inputs.read().await;
                    let Some(input_state) = inputs.get(&input_port_id) else {
                        todo!();
                    };
                    let mut input_state = input_state.write().await;

                    use ZmqInputPortState::*;
                    match &*input_state {
                        Open(_) => (),
                        Connected(_, _, _, _, connected_ids) => {
                            if connected_ids.iter().any(|&id| id == output_port_id) {
                                #[cfg(feature = "tracing")]
                                span.in_scope(|| {
                                    trace!(?output_port_id, "output port is already connected")
                                });
                                return;
                            }
                        }
                        Closed => return,
                    };

                    let add_connection = |input_state: &mut ZmqInputPortState| match input_state {
                        Open(to_worker_send) => {
                            let (msgs_send, msgs_recv) = sync_channel(1);
                            let msgs_recv = Arc::new(Mutex::new(msgs_recv));
                            *input_state = Connected(
                                req_send.clone(),
                                msgs_send,
                                msgs_recv,
                                to_worker_send.clone(),
                                vec![output_port_id],
                            );
                        }
                        Connected(_, _, _, _, ids) => {
                            ids.push(output_port_id);
                        }
                        Closed => unreachable!(),
                    };

                    pub_queue
                        .send(ZmqTransportEvent::AckConnection(
                            output_port_id,
                            input_port_id,
                        ))
                        .await
                        .expect("input worker send ack-conn event");

                    #[cfg(feature = "tracing")]
                    span.in_scope(|| trace!(?output_port_id, "sent conn-ack"));

                    add_connection(&mut input_state);

                    #[cfg(feature = "tracing")]
                    span.in_scope(|| trace!(?input_state, "connected new port"));
                }
                Message(output_port_id, _, seq_id, bytes) => {
                    #[cfg(feature = "tracing")]
                    let span = trace_span!(parent: &span, "Message", ?output_port_id, ?seq_id);

                    let inputs = inputs.read().await;
                    let Some(input_state) = inputs.get(&input_port_id) else {
                        todo!();
                    };
                    let input_state = input_state.read().await;

                    use ZmqInputPortState::*;
                    match &*input_state {
                        Connected(_, sender, _, _, connected_ids) => {
                            if !connected_ids.iter().any(|id| *id == output_port_id) {
                                #[cfg(feature = "tracing")]
                                span.in_scope(|| {
                                    trace!("got message from non-connected output port")
                                });
                                return;
                            }

                            sender
                                .send(ZmqInputPortEvent::Message(bytes))
                                .await
                                .expect("input worker send message");

                            pub_queue
                                .send(ZmqTransportEvent::AckMessage(
                                    output_port_id,
                                    input_port_id,
                                    seq_id,
                                ))
                                .await
                                .expect("input worker send message ack");

                            #[cfg(feature = "tracing")]
                            span.in_scope(|| trace!("sent msg-ack"));
                        }

                        Open(_) | Closed => todo!(),
                    }
                }
                CloseOutput(output_port_id, input_port_id) => {
                    let inputs = inputs.read().await;
                    let Some(input_state) = inputs.get(&input_port_id) else {
                        todo!();
                    };

                    let mut input_state = input_state.write().await;

                    use ZmqInputPortState::*;
                    let Connected(_, _, _, _, ref connected_ids) = *input_state else {
                        return;
                    };

                    if !connected_ids.iter().any(|id| *id == output_port_id) {
                        #[cfg(feature = "tracing")]
                        span.in_scope(|| {
                            trace!(
                                ?output_port_id,
                                "output port doesn't match any connected port"
                            )
                        });
                        return;
                    }

                    match *input_state {
                        Open(_) | Closed => (),
                        Connected(_, ref sender, _, _, ref mut connected_ids) => {
                            connected_ids.retain(|&id| id != output_port_id);
                            if connected_ids.is_empty() {
                                #[cfg(feature = "tracing")]
                                span.in_scope(|| trace!("last connected port disconnected"));
                                sender
                                    .send(ZmqInputPortEvent::Closed)
                                    .await
                                    .expect("input worker publish Closed event");
                            }
                        }
                    };
                }

                // ignore, ideally we never receive these here:
                AckConnection(_, _) | AckMessage(_, _, _) | CloseInput(_) => (),
            }
        }

        async fn handle_input_request(
            request: ZmqInputPortRequest,
            response_chan: Sender<Result<(), PortError>>,
            inputs: &RwLock<BTreeMap<InputPortID, RwLock<ZmqInputPortState>>>,
            pub_queue: &Sender<ZmqTransportEvent>,
            sub_queue: &Sender<ZmqSubscriptionRequest>,
            input_port_id: InputPortID,
        ) {
            #[cfg(feature = "tracing")]
            let span = trace_span!(
                "ZmqTransport::start_input_worker::handle_input_event",
                ?input_port_id
            );

            #[cfg(feature = "tracing")]
            span.in_scope(|| trace!(?request, "got input request"));

            use ZmqInputPortRequest::*;
            match request {
                Close => {
                    let inputs = inputs.read().await;
                    let Some(input_state) = inputs.get(&input_port_id) else {
                        todo!();
                    };
                    let mut input_state = input_state.write().await;

                    use ZmqInputPortState::*;
                    let Connected(_, ref port_events, _, _, _) = *input_state else {
                        return;
                    };

                    pub_queue
                        .send(ZmqTransportEvent::CloseInput(input_port_id))
                        .await
                        .expect("input worker send close event");

                    port_events
                        .send(ZmqInputPortEvent::Closed)
                        .await
                        .expect("input worker send port closed");

                    *input_state = ZmqInputPortState::Closed;

                    {
                        let mut handles = Vec::new();
                        for topic in input_topics(input_port_id).into_iter() {
                            #[cfg(feature = "tracing")]
                            span.in_scope(|| trace!(?topic, "sending unsubscription request"));

                            let handle = sub_queue.send(ZmqSubscriptionRequest::Unsubscribe(topic));
                            handles.push(handle);
                        }
                        for handle in handles.into_iter() {
                            handle.await.expect("input worker send unsub req");
                        }
                    }

                    response_chan
                        .send(Ok(()))
                        .await
                        .expect("input worker respond close")
                }
            }
        }

        #[cfg(feature = "tracing")]
        span.in_scope(|| trace!("spawning"));

        tokio::task::spawn(async move {
            // Input worker loop:
            //   1. Receive connection attempts and respond
            //   2. Receive messages and forward to channel
            //   3. Receive and handle disconnects
            loop {
                tokio::select! {
                    Some(event) = to_worker_recv.recv() => {
                        handle_socket_event(event, &inputs, &req_send, &pub_queue, input_port_id).await;
                    }
                    Some((request, response_chan)) = req_recv.recv() => {
                        handle_input_request(request, response_chan, &inputs, &pub_queue, &sub_queue, input_port_id).await;
                    }
                };
            }
        });

        Ok(())
    }

    fn start_output_worker(&self, output_port_id: OutputPortID) -> Result<(), PortError> {
        #[cfg(feature = "tracing")]
        let span = trace_span!("ZmqTransport::start_output_worker", ?output_port_id);

        let (conn_send, mut conn_recv) = sync_channel(1);

        let (to_worker_send, mut to_worker_recv) = sync_channel(1);

        {
            let mut outputs = self.tokio.block_on(self.outputs.write());

            if outputs.contains_key(&output_port_id) {
                return Ok(()); // TODO
            }
            let state = ZmqOutputPortState::Open(conn_send, to_worker_send.clone());
            let state = RwLock::new(state);

            #[cfg(feature = "tracing")]
            span.in_scope(|| trace!(?state, "saving new opened state"));

            outputs.insert(output_port_id, state);
        }

        let sub_queue = self.sub_queue.clone();
        let pub_queue = self.pub_queue.clone();
        let outputs = self.outputs.clone();

        #[cfg(feature = "tracing")]
        span.in_scope(|| trace!("spawning"));

        tokio::task::spawn(async move {
            let Some((input_port_id, conn_confirm)) = conn_recv.recv().await else {
                todo!();
            };

            #[cfg(feature = "tracing")]
            let span = trace_span!(
                "ZmqTransport::start_output_worker::spawn",
                ?output_port_id,
                ?input_port_id
            );

            {
                let mut handles = Vec::new();
                for topic in output_topics(output_port_id, input_port_id).into_iter() {
                    #[cfg(feature = "tracing")]
                    span.in_scope(|| trace!(?topic, "sending subscription request"));

                    let handle = sub_queue.send(ZmqSubscriptionRequest::Subscribe(topic));
                    handles.push(handle);
                }
                for handle in handles.into_iter() {
                    handle.await.expect("output worker send sub req");
                }
            }

            let (msg_req_send, mut msg_req_recv) = sync_channel(1);

            // Output worker loop:
            //   1. Send connection attempt
            //   2. Send messages
            //     2.1 Wait for ACK
            //     2.2. Resend on timeout
            //   3. Send disconnect events

            loop {
                #[cfg(feature = "tracing")]
                span.in_scope(|| trace!("sending connection attempt..."));

                pub_queue
                    .send(ZmqTransportEvent::Connect(output_port_id, input_port_id))
                    .await
                    .expect("output worker send connect event");

                let response = to_worker_recv
                    .recv()
                    .await
                    .expect("output worker recv ack-conn event");

                #[cfg(feature = "tracing")]
                span.in_scope(|| trace!(?response, "got response"));

                use ZmqTransportEvent::*;
                match response {
                    AckConnection(_, input_port_id) => {
                        let outputs = outputs.read().await;
                        let Some(output_state) = outputs.get(&output_port_id) else {
                            todo!();
                        };
                        let mut output_state = output_state.write().await;
                        debug_assert!(matches!(*output_state, ZmqOutputPortState::Open(..)));
                        *output_state = ZmqOutputPortState::Connected(
                            msg_req_send,
                            to_worker_send,
                            input_port_id,
                        );

                        #[cfg(feature = "tracing")]
                        span.in_scope(|| trace!(?output_state, "Connected!"));

                        conn_confirm
                            .send(Ok(()))
                            .await
                            .expect("output worker respond conn");
                        drop(conn_confirm);

                        break;
                    }
                    _ => continue,
                }
            }

            let mut seq_id = 1;
            'send: loop {
                #[cfg(feature = "tracing")]
                let span = trace_span!(parent: &span, "send_loop", ?seq_id);

                let (request, response_chan) = msg_req_recv
                    .recv()
                    .await
                    .expect("output worker recv msg req");

                #[cfg(feature = "tracing")]
                span.in_scope(|| trace!(?request, "sending request"));

                match request {
                    ZmqOutputPortRequest::Close => {
                        let response = pub_queue
                            .send(ZmqTransportEvent::CloseOutput(
                                output_port_id,
                                input_port_id,
                            ))
                            .await
                            .map_err(|e| PortError::Other(e.to_string()));

                        {
                            let mut handles = Vec::new();
                            for topic in output_topics(output_port_id, input_port_id).into_iter() {
                                #[cfg(feature = "tracing")]
                                span.in_scope(|| trace!(?topic, "sending unsubscription request"));
                                let handle =
                                    sub_queue.send(ZmqSubscriptionRequest::Unsubscribe(topic));
                                handles.push(handle);
                            }
                            for handle in handles.into_iter() {
                                handle.await.expect("output worker send unsub req");
                            }
                        }

                        response_chan
                            .send(response)
                            .await
                            .expect("output worker respond close");
                    }
                    ZmqOutputPortRequest::Send(bytes) => {
                        pub_queue
                            .send(ZmqTransportEvent::Message(
                                output_port_id,
                                input_port_id,
                                seq_id,
                                bytes,
                            ))
                            .await
                            .expect("output worker send message event");

                        'recv: loop {
                            let event = to_worker_recv
                                .recv()
                                .await
                                .expect("output worker event recv");

                            #[cfg(feature = "tracing")]
                            span.in_scope(|| trace!(?event, "received event"));

                            use ZmqTransportEvent::*;
                            match event {
                                AckMessage(_, _, ack_id) => {
                                    if ack_id == seq_id {
                                        #[cfg(feature = "tracing")]
                                        span.in_scope(|| trace!(?ack_id, "msg-ack matches"));
                                        response_chan
                                            .send(Ok(()))
                                            .await
                                            .expect("output worker respond send");
                                        break 'recv;
                                    }
                                }
                                CloseInput(_) => {
                                    let outputs = outputs.read().await;
                                    let Some(output_state) = outputs.get(&output_port_id) else {
                                        todo!();
                                    };
                                    let mut output_state = output_state.write().await;
                                    debug_assert!(matches!(
                                        *output_state,
                                        ZmqOutputPortState::Connected(..)
                                    ));
                                    *output_state = ZmqOutputPortState::Closed;

                                    {
                                        let mut handles = Vec::new();
                                        for topic in
                                            output_topics(output_port_id, input_port_id).into_iter()
                                        {
                                            #[cfg(feature = "tracing")]
                                            span.in_scope(|| {
                                                trace!(?topic, "sending unsubscription request")
                                            });
                                            let handle = sub_queue
                                                .send(ZmqSubscriptionRequest::Unsubscribe(topic));
                                            handles.push(handle);
                                        }
                                        for handle in handles.into_iter() {
                                            handle.await.expect("output worker send unsub req");
                                        }
                                    }

                                    response_chan
                                        .send(Err(PortError::Disconnected))
                                        .await
                                        .expect("output worker respond msg");

                                    break 'send;
                                }

                                // ignore others, we shouldn't receive any new conn-acks
                                // nor should we be receiving input port events
                                AckConnection(_, _)
                                | Connect(_, _)
                                | Message(_, _, _, _)
                                | CloseOutput(_, _) => continue,
                            }
                        }
                    }
                }

                seq_id += 1;
            }
        });

        Ok(())
    }
}

impl Transport for ZmqTransport {
    fn input_state(&self, input: InputPortID) -> PortResult<PortState> {
        self.tokio.block_on(async {
            Ok(self
                .inputs
                .read()
                .await
                .get(&input)
                .ok_or_else(|| PortError::Invalid(input.into()))?
                .read()
                .await
                .state())
        })
    }

    fn output_state(&self, output: OutputPortID) -> PortResult<PortState> {
        self.tokio.block_on(async {
            Ok(self
                .outputs
                .read()
                .await
                .get(&output)
                .ok_or_else(|| PortError::Invalid(output.into()))?
                .read()
                .await
                .state())
        })
    }

    fn open_input(&self) -> PortResult<InputPortID> {
        #[cfg(feature = "tracing")]
        trace!(target: "ZmqTransport::open_input", "creating new input port");

        let new_id = {
            let inputs = self.tokio.block_on(self.inputs.read());
            InputPortID::try_from(-(inputs.len() as isize + 1))
                .map_err(|e| PortError::Other(e.to_string()))?
        };

        #[cfg(feature = "tracing")]
        trace!(target: "ZmqTransport::open_input", ?new_id, "created new input port");

        self.start_input_worker(new_id).map(|_| new_id)
    }

    fn open_output(&self) -> PortResult<OutputPortID> {
        #[cfg(feature = "tracing")]
        trace!(target: "ZmqTransport::open_output", "creating new output port");

        let new_id = {
            let outputs = self.tokio.block_on(self.outputs.read());
            OutputPortID::try_from(outputs.len() as isize + 1)
                .map_err(|e| PortError::Other(e.to_string()))?
        };

        #[cfg(feature = "tracing")]
        trace!(target: "ZmqTransport::open_output", ?new_id, "created new output port");

        self.start_output_worker(new_id).map(|_| new_id)
    }

    fn close_input(&self, input: InputPortID) -> PortResult<bool> {
        self.tokio.block_on(async {
            let sender = {
                let inputs = self.inputs.read().await;
                let Some(input_state) = inputs.get(&input) else {
                    return Err(PortError::Invalid(input.into()));
                };

                let input_state = input_state.read().await;
                let ZmqInputPortState::Connected(sender, _, _, _, _) = &*input_state else {
                    return Err(PortError::Disconnected);
                };

                sender.clone()
            };

            let (close_send, mut close_recv) = sync_channel(1);

            sender
                .send((ZmqInputPortRequest::Close, close_send))
                .await
                .map_err(|e| PortError::Other(e.to_string()))?;

            close_recv
                .recv()
                .await
                .ok_or(PortError::Disconnected)?
                .map(|_| true)
        })
    }

    fn close_output(&self, output: OutputPortID) -> PortResult<bool> {
        self.tokio.block_on(async {
            let sender = {
                let outputs = self.outputs.read().await;
                let Some(output_state) = outputs.get(&output) else {
                    return Err(PortError::Invalid(output.into()));
                };

                let output_state = output_state.read().await;
                let ZmqOutputPortState::Connected(sender, _, _) = &*output_state else {
                    return Err(PortError::Disconnected);
                };

                sender.clone()
            };

            let (close_send, mut close_recv) = sync_channel(1);

            sender
                .send((ZmqOutputPortRequest::Close, close_send))
                .await
                .map_err(|e| PortError::Other(e.to_string()))?;

            close_recv
                .recv()
                .await
                .ok_or(PortError::Disconnected)?
                .map(|_| true)
        })
    }

    fn connect(&self, source: OutputPortID, target: InputPortID) -> PortResult<bool> {
        #[cfg(feature = "tracing")]
        trace!(target: "ZmqTransport::connect", ?source, ?target, "connecting ports");

        self.tokio.block_on(async {
            let sender = {
                let outputs = self.outputs.read().await;
                let Some(output_state) = outputs.get(&source) else {
                    return Err(PortError::Invalid(source.into()));
                };

                let output_state = output_state.read().await;
                let ZmqOutputPortState::Open(ref sender, _) = *output_state else {
                    return Err(PortError::Invalid(source.into()));
                };

                sender.clone()
            };

            let (confirm_send, mut confirm_recv) = sync_channel(1);

            sender
                .send((target, confirm_send))
                .await
                .map_err(|e| PortError::Other(e.to_string()))?;

            confirm_recv
                .recv()
                .await
                .ok_or(PortError::Disconnected)?
                .map(|_| true)
        })
    }

    fn send(&self, output: OutputPortID, message: Bytes) -> PortResult<()> {
        #[cfg(feature = "tracing")]
        trace!(target: "ZmqTransport::send", ?output, "sending from output port");

        self.tokio.block_on(async {
            let sender = {
                let outputs = self.outputs.read().await;
                let Some(output) = outputs.get(&output) else {
                    return Err(PortError::Invalid(output.into()));
                };
                let output = output.read().await;

                let ZmqOutputPortState::Connected(sender, _, _) = &*output else {
                    return Err(PortError::Disconnected);
                };

                sender.clone()
            };

            let (ack_send, mut ack_recv) = sync_channel(1);

            sender
                .send((ZmqOutputPortRequest::Send(message), ack_send))
                .await
                .map_err(|e| PortError::Other(e.to_string()))?;

            ack_recv.recv().await.ok_or(PortError::Disconnected)?
        })
    }

    fn recv(&self, input: InputPortID) -> PortResult<Option<Bytes>> {
        #[cfg(feature = "tracing")]
        trace!(target: "ZmqTransport::recv", ?input, "receiving from input port");

        self.tokio.block_on(async {
            let receiver = {
                let inputs = self.inputs.read().await;
                let Some(input_state) = inputs.get(&input) else {
                    return Err(PortError::Invalid(input.into()));
                };

                let input_state = input_state.read().await;
                let ZmqInputPortState::Connected(_, _, receiver, _, _) = &*input_state else {
                    return Err(PortError::Disconnected);
                };

                receiver.clone()
            };

            let mut receiver = receiver.lock().await;

            use ZmqInputPortEvent::*;
            match receiver.recv().await {
                Some(Closed) => Ok(None), // EOS
                Some(Message(bytes)) => Ok(Some(bytes)),
                None => Err(PortError::Disconnected),
            }
        })
    }

    fn try_recv(&self, input: InputPortID) -> PortResult<Option<Bytes>> {
        #[cfg(feature = "tracing")]
        trace!(target: "ZmqTransport::try_recv", ?input, "receiving from input port");

        self.tokio.block_on(async {
            let receiver = {
                let inputs = self.inputs.read().await;
                let Some(input_state) = inputs.get(&input) else {
                    return Err(PortError::Invalid(input.into()));
                };

                let input_state = input_state.read().await;
                let ZmqInputPortState::Connected(_, _, receiver, _, _) = &*input_state else {
                    return Err(PortError::Disconnected);
                };

                receiver.clone()
            };

            let mut receiver = receiver.lock().await;

            use ZmqInputPortEvent::*;
            match receiver.try_recv() {
                Ok(Closed) => Ok(None), // EOS
                Ok(Message(bytes)) => Ok(Some(bytes)),
                Err(TryRecvError::Disconnected) => Err(PortError::Disconnected),
                // TODO: what should we answer with here?:
                Err(TryRecvError::Empty) => Err(PortError::RecvFailed),
            }
        })
    }
}

async fn handle_zmq_msg(
    msg: ZmqMessage,
    outputs: &RwLock<BTreeMap<OutputPortID, RwLock<ZmqOutputPortState>>>,
    inputs: &RwLock<BTreeMap<InputPortID, RwLock<ZmqInputPortState>>>,
) -> Result<(), Error> {
    let Ok(event) = ZmqTransportEvent::try_from(msg) else {
        todo!();
    };

    #[cfg(feature = "tracing")]
    trace!(target: "handle_zmq_msg", ?event, "got event");

    use ZmqTransportEvent::*;
    match event {
        // input ports
        Connect(_, input_port_id) => {
            let sender = {
                let inputs = inputs.read().await;
                let Some(input) = inputs.get(&input_port_id) else {
                    todo!();
                };
                let input = input.read().await;

                use ZmqInputPortState::*;
                match &*input {
                    Closed => todo!(),
                    Open(sender) | Connected(_, _, _, sender, _) => sender.clone(),
                }
            };

            sender.send(event).await.unwrap();
        }
        Message(_, input_port_id, _, _) => {
            let sender = {
                let inputs = inputs.read().await;
                let Some(input) = inputs.get(&input_port_id) else {
                    todo!();
                };

                let input = input.read().await;
                let ZmqInputPortState::Connected(_, _, _, sender, _) = &*input else {
                    todo!();
                };

                sender.clone()
            };

            sender.send(event).await.unwrap();
        }
        CloseOutput(_, input_port_id) => {
            let sender = {
                let inputs = inputs.read().await;
                let Some(input) = inputs.get(&input_port_id) else {
                    todo!();
                };
                let input = input.read().await;

                use ZmqInputPortState::*;
                match &*input {
                    Closed => todo!(),
                    Open(sender) | Connected(_, _, _, sender, _) => sender.clone(),
                }
            };

            sender.send(event).await.unwrap();
        }

        // output ports
        AckConnection(output_port_id, _) => {
            let sender = {
                let outputs = outputs.read().await;
                let Some(output) = outputs.get(&output_port_id) else {
                    todo!();
                };
                let output = output.read().await;

                let ZmqOutputPortState::Open(_, sender) = &*output else {
                    todo!();
                };

                sender.clone()
            };

            sender.send(event).await.unwrap();
        }
        AckMessage(output_port_id, _, _) => {
            let sender = {
                let outputs = outputs.read().await;
                let Some(output) = outputs.get(&output_port_id) else {
                    todo!();
                };
                let output = output.read().await;
                let ZmqOutputPortState::Connected(_, sender, _) = &*output else {
                    todo!();
                };

                sender.clone()
            };

            sender.send(event).await.unwrap();
        }
        CloseInput(input_port_id) => {
            for (_, state) in outputs.read().await.iter() {
                let sender = {
                    let state = state.read().await;
                    let ZmqOutputPortState::Connected(_, ref sender, ref id) = *state else {
                        continue;
                    };
                    if *id != input_port_id {
                        continue;
                    }

                    sender.clone()
                };

                if let Err(_e) = sender.send(event.clone()).await {
                    continue; // TODO
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use protoflow_core::{runtimes::StdRuntime, System};

    use futures_util::future::TryFutureExt;
    use zeromq::{PubSocket, SocketRecv, SocketSend, SubSocket};

    async fn start_zmqtransport_server() {
        // bind a *SUB* socket to the *PUB* address so that the transport can *PUB* to it
        let mut pub_srv = SubSocket::new();
        pub_srv.bind(DEFAULT_PUB_SOCKET).await.unwrap();

        // bind a *PUB* socket to the *SUB* address so that the transport can *SUB* to it
        let mut sub_srv = PubSocket::new();
        sub_srv.bind(DEFAULT_SUB_SOCKET).await.unwrap();

        // subscribe to all messages
        pub_srv.subscribe("").await.unwrap();

        // resend anything received on the *SUB* socket to the *PUB* socket
        tokio::task::spawn(async move {
            let mut pub_srv = pub_srv;
            loop {
                pub_srv
                    .recv()
                    .and_then(|msg| sub_srv.send(msg))
                    .await
                    .unwrap();
            }
        });
    }

    #[test]
    fn implementation_matches() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();

        //zeromq::proxy(frontend, backend, capture)
        rt.block_on(start_zmqtransport_server());

        let _ = System::<ZmqTransport>::build(|_s| { /* do nothing */ });
    }

    #[test]
    fn run_transport() {
        tracing_subscriber::fmt::init();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();

        rt.block_on(start_zmqtransport_server());

        let transport = ZmqTransport::default();
        let runtime = StdRuntime::new(transport).unwrap();
        let system = System::new(&runtime);

        let output = system.output();
        let input = system.input();

        system.connect(&output, &input);

        let output = std::thread::spawn(move || {
            let mut output = output;
            output.send(&"Hello world!".to_string())?;
            output.close()
        });

        let input = std::thread::spawn(move || {
            let mut input = input;

            let msg = input.recv()?;
            assert_eq!(Some("Hello world!".to_string()), msg);

            let msg = input.recv()?;
            assert_eq!(None, msg);

            input.close()
        });

        output.join().expect("thread failed").unwrap();
        input.join().expect("thread failed").unwrap();
    }
}
