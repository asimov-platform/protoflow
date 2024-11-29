// This is free and unencumbered software released into the public domain.

#![no_std]
#![deny(unsafe_code)]

#[doc(hidden)]
pub use protoflow_core::prelude;

extern crate std;

use protoflow_core::{
    prelude::{Arc, BTreeMap, Bytes, String, ToString, Vec},
    InputPortID, OutputPortID, PortError, PortResult, PortState, Transport,
};

use core::fmt::Error;
use parking_lot::{Mutex, RwLock};
use std::{
    format,
    sync::mpsc::{sync_channel, Receiver, SyncSender},
    write,
};
use zeromq::{
    util::PeerIdentity, Socket, SocketOptions, SocketRecv, SocketSend, ZmqError, ZmqMessage,
};

const DEFAULT_PUB_SOCKET: &str = "tcp://127.0.0.1:10000";
const DEFAULT_SUB_SOCKET: &str = "tcp://127.0.0.1:10001";

pub struct ZmqTransport {
    tokio: tokio::runtime::Handle,

    pub_queue: Arc<SyncSender<ZmqTransportEvent>>,
    sub_queue: Arc<tokio::sync::mpsc::Sender<ZmqSubscriptionRequest>>,

    outputs: Arc<RwLock<BTreeMap<OutputPortID, RwLock<ZmqOutputPortState>>>>,
    inputs: Arc<RwLock<BTreeMap<InputPortID, RwLock<ZmqInputPortState>>>>,
}

#[derive(Debug, Clone)]
enum ZmqOutputPortState {
    Open(Arc<SyncSender<(InputPortID, SyncSender<Result<(), PortError>>)>>),
    Connected(
        // channels for public send, contained channel is for the ack back
        Arc<SyncSender<(ZmqOutputPortRequest, SyncSender<Result<(), PortError>>)>>,
        // internal channels for events
        Arc<SyncSender<ZmqTransportEvent>>,
        Arc<Mutex<Receiver<ZmqOutputPortEvent>>>,
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
            Open(_) => PortState::Open,
            Connected(_, _, _, _) => PortState::Connected,
            Closed => PortState::Closed,
        }
    }
}

#[derive(Debug, Clone)]
enum ZmqInputPortState {
    Open(
        // TODO: hide these
        Arc<SyncSender<ZmqTransportEvent>>,
        Arc<Mutex<Receiver<ZmqInputPortEvent>>>,
    ),
    Connected(
        // channels for requests from public close
        Arc<SyncSender<(ZmqInputPortRequest, SyncSender<Result<(), PortError>>)>>,
        Arc<Mutex<Receiver<(ZmqInputPortRequest, SyncSender<Result<(), PortError>>)>>>,
        // channels for the public recv
        Arc<SyncSender<ZmqInputPortEvent>>,
        Arc<Mutex<Receiver<ZmqInputPortEvent>>>,
        // internal  channels for events
        Arc<SyncSender<ZmqTransportEvent>>,
        Arc<Mutex<Receiver<ZmqInputPortEvent>>>,
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
            Open(_, _) => PortState::Open,
            Connected(_, _, _, _, _, _, _) => PortState::Connected,
            Closed => PortState::Closed,
        }
    }
}

type SequenceID = u64;

/// ZmqTransportEvent represents the data that goes over the wire, sent from an output port over
/// the network to an input port.
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
            Message(o, i, seq, _payload) => write!(f, "{}:msg:{}:{}", i, o, seq),
            AckMessage(o, i, seq) => write!(f, "{}:ackMsg:{}:{}", i, o, seq),
            CloseOutput(o, i) => write!(f, "{}:closeOut:{}", i, o),
            CloseInput(i) => write!(f, "{}:closeIn", i),
        }
    }
}

impl From<ZmqTransportEvent> for ZmqMessage {
    fn from(value: ZmqTransportEvent) -> Self {
        let mut topic = Vec::new();
        value.write_topic(&mut topic).unwrap();

        // first frame of the message is the topic
        let mut msg = ZmqMessage::from(topic.clone());

        // second frame of the message is the payload
        use ZmqTransportEvent::*;
        match value {
            Connect(output_port_id, input_port_id) => todo!(),
            AckConnection(output_port_id, input_port_id) => todo!(),
            Message(_, _, _, bytes) => msg.push_back(bytes),
            AckMessage(output_port_id, input_port_id, _) => todo!(),
            CloseOutput(output_port_id, input_port_id) => todo!(),
            CloseInput(input_port_id) => todo!(),
        };

        msg
    }
}

impl TryFrom<ZmqMessage> for ZmqTransportEvent {
    type Error = protoflow_core::DecodeError;

    fn try_from(value: ZmqMessage) -> Result<Self, Self::Error> {
        todo!()
    }
}

/// ZmqOutputPortEvent represents events that we receive from the background worker of the port.
#[derive(Clone, Debug)]
enum ZmqOutputPortEvent {
    Opened,
    Connected(InputPortID),
    Ack(SequenceID),
    Closed,
}

/// ZmqInputPortEvent represents events that we receive from the background worker of the port.
#[derive(Clone, Debug)]
enum ZmqInputPortEvent {
    Opened,
    Connected(OutputPortID),
    Message(Bytes),
    Closed,
}

impl Default for ZmqTransport {
    fn default() -> Self {
        Self::new(DEFAULT_PUB_SOCKET, DEFAULT_SUB_SOCKET)
    }
}

#[derive(Clone)]
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

        let (out_queue, out_queue_recv) = sync_channel(1);

        let out_queue = Arc::new(out_queue);

        let (sub_queue, sub_queue_recv) = tokio::sync::mpsc::channel(1);
        let sub_queue = Arc::new(sub_queue);

        let transport = Self {
            pub_queue: out_queue,
            sub_queue,
            tokio,
            outputs,
            inputs,
        };

        transport.start_send_worker(psock, out_queue_recv);
        transport.start_recv_worker(ssock, sub_queue_recv);

        transport
    }

    fn start_send_worker(&self, psock: zeromq::PubSocket, queue: Receiver<ZmqTransportEvent>) {
        let tokio = self.tokio.clone();
        let mut psock = psock;

        tokio::task::spawn(async move {
            queue
                .into_iter()
                .map(ZmqMessage::from)
                .try_for_each(|msg| tokio.block_on(psock.send(msg)))
                .expect("zmq send worker")
        });
    }

    fn start_recv_worker(
        &self,
        ssock: zeromq::SubSocket,
        sub_queue: tokio::sync::mpsc::Receiver<ZmqSubscriptionRequest>,
    ) {
        let mut ssock = ssock;
        let mut sub_queue = sub_queue;

        let outputs = self.outputs.clone();
        let inputs = self.inputs.clone();

        tokio::task::spawn(async move {
            loop {
                tokio::select! {
                    Ok(msg) = ssock.recv() => handle_zmq_msg(msg, &outputs, &inputs).unwrap(),
                    Some(req) = sub_queue.recv() => {
                        use ZmqSubscriptionRequest::*;
                        match req {
                            Subscribe(topic) => ssock.subscribe(&topic).await.expect("zmq recv worker subscribe"),
                            Unsubscribe(topic) => ssock.unsubscribe(&topic).await.expect("zmq recv worker subscribe"),
                        };
                    }
                };
            }
        });
    }

    fn subscribe_for_input_port(
        &self,
        input: InputPortID,
    ) -> Result<
        (
            Arc<SyncSender<ZmqTransportEvent>>,
            Arc<Mutex<Receiver<ZmqInputPortEvent>>>,
        ),
        ZmqError,
    > {
        // TODO: only sub to relevant events
        let topic = format!("{}:", input);
        self.tokio
            .block_on(
                self.sub_queue
                    .send(ZmqSubscriptionRequest::Subscribe(topic)),
            )
            .unwrap();

        let (from_worker_send, from_worker_recv) = sync_channel(1);
        let (to_worker_send, to_worker_recv) = sync_channel(1);

        let to_worker_send = Arc::new(to_worker_send);
        let from_worker_recv = Arc::new(Mutex::new(from_worker_recv));

        // Input worker loop:
        //   1. Receive connection attempts and respond
        //   2. Receive messages and forward to channel
        //   3. Receive and handle disconnects
        {
            let inputs = self.inputs.clone();

            let to_worker_send = to_worker_send.clone();
            let from_worker_recv = from_worker_recv.clone();

            tokio::task::spawn(async move {
                let (output, input) = (from_worker_send, to_worker_recv);

                loop {
                    use ZmqTransportEvent::*;
                    let event = input.recv().expect("input worker recv");
                    match event {
                        // Connection attempt
                        Connect(_, input_port_id) => {
                            let inputs = inputs.read();
                            let Some(input) = inputs.get(&input_port_id) else {
                                todo!();
                            };

                            let (req_send, req_recv) = sync_channel(1);
                            let req_send = Arc::new(req_send);
                            let req_recv = Arc::new(Mutex::new(req_recv));

                            let (msgs_send, msgs_recv) = sync_channel(1);

                            let msgs_send = Arc::new(msgs_send);
                            let msgs_recv = Arc::new(Mutex::new(msgs_recv));

                            let mut input = input.write();
                            *input = ZmqInputPortState::Connected(
                                req_send,
                                req_recv,
                                msgs_send,
                                msgs_recv,
                                to_worker_send.clone(),
                                from_worker_recv.clone(),
                                Vec::new(),
                            );
                        }

                        // Message from output port
                        Message(_, input_port_id, _, bytes) => {
                            let inputs = inputs.read();
                            let Some(input) = inputs.get(&input_port_id) else {
                                todo!();
                            };

                            let input = input.read();
                            use ZmqInputPortState::*;
                            match &*input {
                                Open(_, _) => todo!(),
                                Closed => todo!(),
                                Connected(_, _, sender, _, _, _, _) => {
                                    sender.send(ZmqInputPortEvent::Message(bytes)).unwrap()
                                }
                            };
                        }

                        // Output port reports being closed
                        CloseInput(input_port_id) => {
                            let inputs = inputs.read();
                            let Some(input) = inputs.get(&input_port_id) else {
                                todo!();
                            };

                            let input = input.read();
                            use ZmqInputPortState::*;
                            match &*input {
                                Open(_, _) => todo!(),
                                Closed => todo!(),
                                Connected(_, _, sender, _, _, _, _) => {
                                    sender.send(ZmqInputPortEvent::Closed).unwrap()
                                }
                            };
                        }

                        // ignore output port type events:
                        AckConnection(_, _) | AckMessage(_, _, _) | CloseOutput(_, _) => continue,
                    };
                }
            });
        }

        Ok((to_worker_send, from_worker_recv))
    }
}

impl Transport for ZmqTransport {
    fn input_state(&self, input: InputPortID) -> PortResult<PortState> {
        self.inputs
            .read()
            .get(&input)
            .map(|port| port.read().state())
            .ok_or(PortError::Invalid(input.into()))
    }

    fn output_state(&self, output: OutputPortID) -> PortResult<PortState> {
        self.outputs
            .read()
            .get(&output)
            .map(|port| port.read().state())
            .ok_or(PortError::Invalid(output.into()))
    }

    fn open_input(&self) -> PortResult<InputPortID> {
        let inputs = self.inputs.write();

        let new_id = InputPortID::try_from(-(inputs.len() as isize + 1))
            .map_err(|e| PortError::Other(e.to_string()))?;

        let (_, receiver) = self
            .subscribe_for_input_port(new_id)
            .map_err(|e| PortError::Other(e.to_string()))?;

        loop {
            let msg = receiver
                .lock()
                .recv()
                .map_err(|e| PortError::Other(e.to_string()))?;
            match msg {
                ZmqInputPortEvent::Opened => break Ok(new_id),
                _ => continue, // TODO
            }
        }
    }

    fn open_output(&self) -> PortResult<OutputPortID> {
        let mut outputs = self.outputs.write();

        let new_id = OutputPortID::try_from(outputs.len() as isize + 1)
            .map_err(|e| PortError::Other(e.to_string()))?;

        let (sender, _receiver) = sync_channel(1);
        let sender = Arc::new(sender);

        let state = RwLock::new(ZmqOutputPortState::Open(sender));
        outputs.insert(new_id, state);

        Ok(new_id)
    }

    fn close_input(&self, input: InputPortID) -> PortResult<bool> {
        let inputs = self.inputs.read();

        let Some(state) = inputs.get(&input) else {
            return Err(PortError::Invalid(input.into()));
        };

        let state = state.read();

        let ZmqInputPortState::Connected(sender, _, _, _, _, _, _) = &*state else {
            return Err(PortError::Disconnected);
        };

        let (close_send, close_recv) = sync_channel(1);

        sender
            .send((ZmqInputPortRequest::Close, close_send))
            .map_err(|e| PortError::Other(e.to_string()))?;

        close_recv
            .recv()
            .map_err(|_| PortError::Disconnected)?
            .map(|_| true)
    }

    fn close_output(&self, output: OutputPortID) -> PortResult<bool> {
        let outputs = self.outputs.read();

        let Some(state) = outputs.get(&output) else {
            return Err(PortError::Invalid(output.into()));
        };

        let state = state.write();

        let ZmqOutputPortState::Connected(_, sender, receiver, input) = &*state else {
            return Err(PortError::Disconnected);
        };

        sender
            .send(ZmqTransportEvent::CloseOutput(output, *input))
            .map_err(|e| PortError::Other(e.to_string()))?;

        loop {
            let msg = receiver
                .lock()
                .recv()
                .map_err(|e| PortError::Other(e.to_string()))?;
            use ZmqOutputPortEvent::*;
            match msg {
                Closed => break Ok(true),
                _ => continue, // TODO
            }
        }
    }

    fn connect(&self, source: OutputPortID, target: InputPortID) -> PortResult<bool> {
        let outputs = self.outputs.read();
        if outputs
            .get(&source)
            .is_some_and(|state| !state.read().state().is_open())
        {
            return Err(PortError::Invalid(source.into()));
        }

        let (from_worker_send, from_worker_recv) = sync_channel::<ZmqOutputPortEvent>(1);
        let (to_worker_send, to_worker_recv) = sync_channel::<ZmqTransportEvent>(1);

        let to_worker_send = Arc::new(to_worker_send);
        let from_worker_recv = Arc::new(Mutex::new(from_worker_recv));

        let (msg_req_send, msg_req_recv) = sync_channel(1);
        let msg_req_send = Arc::new(msg_req_send);
        let msg_req_recv = Arc::new(Mutex::new(msg_req_recv));

        // Output worker loop:
        //   1. Send connection attempt
        //   2. Send messages
        //     2.1 Wait for ACK
        //     2.2. Resend on timeout
        //   3. Send disconnect events
        {
            let to_worker_send = to_worker_send.clone();
            let from_worker_recv = from_worker_recv.clone();

            let pub_queue = self.pub_queue.clone();
            let outputs = self.outputs.clone();

            tokio::task::spawn(async move {
                let (output, input) = (from_worker_send, to_worker_recv);

                // connect loop
                loop {
                    // send request to connect
                    pub_queue
                        .send(ZmqTransportEvent::Connect(source, target))
                        .unwrap();

                    let request = input.recv().expect("output worker recv");
                    match request {
                        ZmqTransportEvent::AckConnection(_, input_port_id) => {
                            let outputs = outputs.read();
                            let Some(output_state) = outputs.get(&source) else {
                                todo!();
                            };
                            let mut output_state = output_state.write();
                            debug_assert!(matches!(*output_state, ZmqOutputPortState::Open(_)));
                            *output_state = ZmqOutputPortState::Connected(
                                msg_req_send,
                                to_worker_send,
                                from_worker_recv,
                                input_port_id,
                            );
                            break;
                        }
                        _ => continue, // TODO: when and why would we receive other events?
                    }
                }

                // TODO: combine these two spawns by using tokio's channels and `select!`

                // work loop for sending events
                tokio::task::spawn(async move {
                    let mut seq_id = 1;
                    loop {
                        let req = msg_req_recv.lock().recv().expect("output worker req recv");

                        let outputs = outputs.read();
                        let Some(output_state) = outputs.get(&source) else {
                            todo!();
                        };

                        let ZmqOutputPortState::Connected(ack_send, sender, _, output_id) =
                            &*output_state.read()
                        else {
                            todo!();
                        };

                        let resp = req.1; // TODO: respond

                        match req.0 {
                            ZmqOutputPortRequest::Send(bytes) => {
                                sender
                                    .send(ZmqTransportEvent::Message(source, target, seq_id, bytes))
                                    .unwrap();
                                seq_id += 1;
                            }
                            ZmqOutputPortRequest::Close => sender
                                .send(ZmqTransportEvent::CloseOutput(source, *output_id))
                                .unwrap(),
                        };
                    }
                });

                // work loop for handling events
                tokio::task::spawn(async move {
                    loop {
                        use ZmqTransportEvent::*;
                        let event = input.recv().expect("output worker event recv");
                        if !matches!(event, Message(_, _, _, _)) {
                            unreachable!("why are we getting non-Message?");
                        }
                        match event {
                            AckMessage(output_port_id, input_port_id, seq_id) => {
                                output
                                    .send(ZmqOutputPortEvent::Ack(seq_id))
                                    .expect("worker loop ack send");
                            }

                            CloseInput(input_port_id) => todo!(),

                            AckConnection(_, _) => {
                                unreachable!("already connected")
                            }

                            // ignore input port type events
                            Connect(_, _) | CloseOutput(_, _) | Message(_, _, _, _) => continue, // TODO
                        }
                    }
                });
            });
        }

        // wait for the `Connected` event
        loop {
            let msg = from_worker_recv
                .lock()
                .recv()
                .map_err(|e| PortError::Other(e.to_string()))?;
            use ZmqOutputPortEvent::*;
            match msg {
                Connected(_) => break Ok(true),
                _ => continue, // TODO
            }
        }
    }

    fn send(&self, output: OutputPortID, message: Bytes) -> PortResult<()> {
        let outputs = self.outputs.read();
        let Some(output) = outputs.get(&output) else {
            return Err(PortError::Invalid(output.into()));
        };
        let output = output.read();

        let ZmqOutputPortState::Connected(sender, _, _, _) = &*output else {
            return Err(PortError::Disconnected);
        };

        let (ack_send, ack_recv) = sync_channel(1);

        sender
            .send((ZmqOutputPortRequest::Send(message), ack_send))
            .unwrap();

        ack_recv.recv().map_err(|_| PortError::Disconnected)?
    }

    fn recv(&self, input: InputPortID) -> PortResult<Option<Bytes>> {
        let inputs = self.inputs.read();
        let Some(input) = inputs.get(&input) else {
            return Err(PortError::Invalid(input.into()));
        };
        let input = input.read();

        let ZmqInputPortState::Connected(_, _, _, receiver, _, _, _) = &*input else {
            return Err(PortError::Disconnected);
        };

        loop {
            use ZmqInputPortEvent::*;
            match receiver.lock().recv() {
                // ignore
                Ok(Opened) | Ok(Connected(_)) => continue,

                Ok(Closed) => break Ok(None), // EOS
                Ok(Message(bytes)) => break Ok(Some(bytes)),
                Err(e) => break Err(PortError::Other(e.to_string())),
            }
        }
    }

    fn try_recv(&self, _input: InputPortID) -> PortResult<Option<Bytes>> {
        todo!();
    }
}

fn handle_zmq_msg(
    msg: ZmqMessage,
    outputs: &RwLock<BTreeMap<OutputPortID, RwLock<ZmqOutputPortState>>>,
    inputs: &RwLock<BTreeMap<InputPortID, RwLock<ZmqInputPortState>>>,
) -> Result<(), Error> {
    let Ok(event) = ZmqTransportEvent::try_from(msg) else {
        todo!();
    };

    use ZmqTransportEvent::*;
    match event {
        // input ports
        Connect(_, input_port_id) => {
            let inputs = inputs.read();
            let Some(input) = inputs.get(&input_port_id) else {
                todo!();
            };
            let input = input.read();

            use ZmqInputPortState::*;
            match &*input {
                Closed => todo!(),
                Open(sender, _) | Connected(_, _, _, _, sender, _, _) => {
                    sender.send(event).unwrap();
                }
            };
        }
        Message(output_port_id, input_port_id, _, bytes) => {
            let inputs = inputs.read();
            let Some(input) = inputs.get(&input_port_id) else {
                todo!();
            };

            let input = input.read();
            let ZmqInputPortState::Connected(_, _, sender, _, _, _, ids) = &*input else {
                todo!();
            };

            // TODO: probably move to ports worker? no sense having here
            if !ids.iter().any(|&id| id == output_port_id) {
                todo!();
            }

            sender.send(ZmqInputPortEvent::Message(bytes)).unwrap();
        }
        CloseOutput(_, input_port_id) => {
            let inputs = inputs.read();
            let Some(input) = inputs.get(&input_port_id) else {
                todo!();
            };
            let input = input.read();

            use ZmqInputPortState::*;
            match &*input {
                Closed => todo!(),
                Open(sender, _) | Connected(_, _, _, _, sender, _, _) => {
                    sender.send(event).unwrap();
                }
            };
        }

        // output ports
        AckConnection(output_port_id, _) => {
            let outputs = outputs.read();
            let Some(output) = outputs.get(&output_port_id) else {
                todo!();
            };
            let output = output.read();

            let ZmqOutputPortState::Open(sender) = &*output else {
                todo!();
            };
            sender.send(event).unwrap();
        }
        AckMessage(output_port_id, _, _) => {
            let outputs = outputs.read();
            let Some(output) = outputs.get(&output_port_id) else {
                todo!();
            };
            let output = output.read();

            let ZmqOutputPortState::Connected(_, sender, _, _) = &*output else {
                todo!();
            };
            sender.send(event).unwrap();
        }
        CloseInput(input_port_id) => {
            let outputs = outputs.read();

            for (_, state) in outputs.iter() {
                let state = state.read();

                let ZmqOutputPortState::Connected(_, sender, _, id) = &*state else {
                    todo!();
                };

                if *id != input_port_id {
                    todo!();
                }

                sender.send(event.clone()).unwrap();
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use protoflow_core::System;

    use futures_util::future::TryFutureExt;
    use zeromq::{PubSocket, SocketRecv, SocketSend, SubSocket};

    fn start_zmqtransport_server(rt: &tokio::runtime::Runtime) {
        // bind a *SUB* socket to the *PUB* address so that the transport can *PUB* to it
        let mut pub_srv = SubSocket::new();
        rt.block_on(pub_srv.bind(DEFAULT_PUB_SOCKET)).unwrap();

        // bind a *PUB* socket to the *SUB* address so that the transport can *SUB* to it
        let mut sub_srv = PubSocket::new();
        rt.block_on(sub_srv.bind(DEFAULT_SUB_SOCKET)).unwrap();

        // subscribe to all messages
        rt.block_on(pub_srv.subscribe("")).unwrap();

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
        start_zmqtransport_server(&rt);

        let _ = System::<ZmqTransport>::build(|_s| { /* do nothing */ });
    }
}
