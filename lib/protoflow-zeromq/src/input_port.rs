// This is free and unencumbered software released into the public domain.

use crate::{
    subscribe_topics, unsubscribe_topics, ZmqSubscriptionRequest, ZmqTransport, ZmqTransportEvent,
};
use protoflow_core::{
    prelude::{fmt, format, vec, Arc, BTreeMap, Bytes, String, ToString, Vec},
    InputPortID, OutputPortID, PortError, PortState,
};
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex, RwLock,
};

#[cfg(feature = "tracing")]
use tracing::{error, info, trace, trace_span, warn};

#[derive(Debug, Clone)]
pub enum ZmqInputPortRequest {
    Close,
}

/// ZmqInputPortEvent represents events that we receive from the background worker of the port.
#[derive(Clone, Debug)]
pub enum ZmqInputPortEvent {
    Message(Bytes),
    Closed,
}

#[derive(Debug, Clone)]
pub enum ZmqInputPortState {
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

impl fmt::Display for ZmqInputPortState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ZmqInputPortState::*;
        match *self {
            Open(..) => write!(f, "Open"),
            Connected(.., ref ids) => {
                write!(
                    f,
                    "Connected({:?})",
                    ids.iter().map(|id| isize::from(*id)).collect::<Vec<_>>()
                )
            }
            Closed => write!(f, "Closed"),
        }
    }
}

impl ZmqInputPortState {
    pub fn state(&self) -> PortState {
        use ZmqInputPortState::*;
        match self {
            Open(_) => PortState::Open,
            Connected(..) => PortState::Connected,
            Closed => PortState::Closed,
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

pub fn start_input_worker(
    transport: &ZmqTransport,
    input_port_id: InputPortID,
) -> Result<(), PortError> {
    #[cfg(feature = "tracing")]
    let span = trace_span!("ZmqTransport::input_port_worker", ?input_port_id);

    let (to_worker_send, mut to_worker_recv) = channel(1);
    let (req_send, mut req_recv) = channel(1);

    {
        let mut inputs = transport.tokio.block_on(transport.inputs.write());
        if inputs.contains_key(&input_port_id) {
            return Err(PortError::Invalid(input_port_id.into()));
        }
        let state = ZmqInputPortState::Open(to_worker_send.clone());
        #[cfg(feature = "tracing")]
        span.in_scope(|| trace!("saving new state: {}", state));
        inputs.insert(input_port_id, RwLock::new(state));
    }

    let sub_queue = transport.sub_queue.clone();
    let pub_queue = transport.pub_queue.clone();
    let inputs = transport.inputs.clone();

    let topics = input_topics(input_port_id);
    if transport
        .tokio
        .block_on(subscribe_topics(&topics, &sub_queue))
        .is_err()
    {
        #[cfg(feature = "tracing")]
        span.in_scope(|| error!("topic subscription failed"));
        return Err(PortError::Other("topic subscription failed".to_string()));
    }

    async fn handle_socket_event(
        event: ZmqTransportEvent,
        inputs: &RwLock<BTreeMap<InputPortID, RwLock<ZmqInputPortState>>>,
        req_send: &Sender<(ZmqInputPortRequest, Sender<Result<(), PortError>>)>,
        pub_queue: &Sender<ZmqTransportEvent>,
        input_port_id: InputPortID,
    ) {
        #[cfg(feature = "tracing")]
        let span = trace_span!(
            "ZmqTransport::input_port_worker::handle_socket_event",
            ?input_port_id
        );

        #[cfg(feature = "tracing")]
        span.in_scope(|| trace!(?event, "got socket event"));

        use ZmqTransportEvent::*;
        match event {
            Connect(output_port_id, input_port_id) => {
                #[cfg(feature = "tracing")]
                let span = trace_span!(parent: &span, "Connect", ?output_port_id);

                let inputs = inputs.read().await;
                let Some(input_state) = inputs.get(&input_port_id) else {
                    #[cfg(feature = "tracing")]
                    span.in_scope(|| error!("port state not found"));
                    return;
                };
                let mut input_state = input_state.write().await;

                use ZmqInputPortState::*;
                match &*input_state {
                    Open(_) => (),
                    Connected(_, _, _, _, connected_ids) => {
                        if connected_ids.iter().any(|&id| id == output_port_id) {
                            #[cfg(feature = "tracing")]
                            span.in_scope(|| trace!("output port is already connected"));
                            return;
                        }
                    }
                    Closed => return,
                };

                let add_connection = |input_state: &mut ZmqInputPortState| match input_state {
                    Open(to_worker_send) => {
                        let (msgs_send, msgs_recv) = channel(1);
                        let msgs_recv = Arc::new(Mutex::new(msgs_recv));
                        *input_state = Connected(
                            req_send.clone(),
                            msgs_send,
                            msgs_recv,
                            to_worker_send.clone(),
                            vec![output_port_id],
                        );
                    }
                    Connected(.., ids) => {
                        ids.push(output_port_id);
                    }
                    Closed => unreachable!(),
                };

                if pub_queue
                    .send(ZmqTransportEvent::AckConnection(
                        output_port_id,
                        input_port_id,
                    ))
                    .await
                    .is_err()
                {
                    #[cfg(feature = "tracing")]
                    span.in_scope(|| warn!("publish channel is closed"));
                    return;
                }

                #[cfg(feature = "tracing")]
                span.in_scope(|| trace!("sent conn-ack"));

                add_connection(&mut input_state);

                #[cfg(feature = "tracing")]
                span.in_scope(|| info!("Connected new port: {}", input_state));
            }
            Message(output_port_id, _, seq_id, bytes) => {
                #[cfg(feature = "tracing")]
                let span = trace_span!(parent: &span, "Message", ?output_port_id, ?seq_id);

                let inputs = inputs.read().await;
                let Some(input_state) = inputs.get(&input_port_id) else {
                    #[cfg(feature = "tracing")]
                    span.in_scope(|| error!("port state not found"));
                    return;
                };
                let input_state = input_state.read().await;

                use ZmqInputPortState::*;
                match &*input_state {
                    Connected(_, sender, _, _, connected_ids) => {
                        if !connected_ids.iter().any(|id| *id == output_port_id) {
                            #[cfg(feature = "tracing")]
                            span.in_scope(|| trace!("got message from non-connected output port"));
                            return;
                        }

                        if sender
                            .send(ZmqInputPortEvent::Message(bytes))
                            .await
                            .is_err()
                        {
                            #[cfg(feature = "tracing")]
                            span.in_scope(|| warn!("receiver for input events has closed"));
                            return;
                        }

                        if pub_queue
                            .send(ZmqTransportEvent::AckMessage(
                                output_port_id,
                                input_port_id,
                                seq_id,
                            ))
                            .await
                            .is_err()
                        {
                            #[cfg(feature = "tracing")]
                            span.in_scope(|| warn!("publish channel is closed"));
                            return;
                        }

                        #[cfg(feature = "tracing")]
                        span.in_scope(|| trace!("sent msg-ack"));
                    }

                    Open(_) | Closed => {
                        #[cfg(feature = "tracing")]
                        span.in_scope(|| warn!("port is not connected: {}", input_state));
                    }
                }
            }
            CloseOutput(output_port_id, input_port_id) => {
                #[cfg(feature = "tracing")]
                let span = trace_span!(parent: &span, "CloseOutput", ?output_port_id);

                let inputs = inputs.read().await;
                let Some(input_state) = inputs.get(&input_port_id) else {
                    #[cfg(feature = "tracing")]
                    span.in_scope(|| error!("port state not found"));
                    return;
                };

                let mut input_state = input_state.write().await;

                use ZmqInputPortState::*;
                let Connected(_, ref sender, _, _, ref mut connected_ids) = *input_state else {
                    #[cfg(feature = "tracing")]
                    span.in_scope(|| trace!("input port wasn't connected"));
                    return;
                };

                let Some(idx) = connected_ids.iter().position(|&id| id == output_port_id) else {
                    #[cfg(feature = "tracing")]
                    span.in_scope(|| trace!("output port doesn't match any connected port"));
                    return;
                };

                connected_ids.swap_remove(idx);

                if !connected_ids.is_empty() {
                    return;
                }

                #[cfg(feature = "tracing")]
                span.in_scope(|| trace!("last connected port disconnected"));

                if let Err(err) = sender.try_send(ZmqInputPortEvent::Closed) {
                    #[cfg(feature = "tracing")]
                    span.in_scope(|| warn!("did not send InputPortEvent::Closed: {}", err));
                }

                // TODO: Should last connection closing close the input port too?
                // It does in the MPSC transport.
                //*input_state = ZmqInputPortState::Closed;
            }

            // ignore, ideally we never receive these here:
            AckConnection(..) | AckMessage(..) | CloseInput(_) => (),
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
            "ZmqTransport::input_port_worker::handle_input_event",
            ?input_port_id
        );

        #[cfg(feature = "tracing")]
        span.in_scope(|| trace!(?request, "got input request"));

        use ZmqInputPortRequest::*;
        match request {
            Close => {
                let inputs = inputs.read().await;
                let Some(input_state) = inputs.get(&input_port_id) else {
                    #[cfg(feature = "tracing")]
                    span.in_scope(|| error!("port state not found"));
                    return;
                };
                let mut input_state = input_state.write().await;

                use ZmqInputPortState::*;
                let Connected(_, ref port_events, _, _, _) = *input_state else {
                    return;
                };

                if pub_queue
                    .send(ZmqTransportEvent::CloseInput(input_port_id))
                    .await
                    .is_err()
                {
                    #[cfg(feature = "tracing")]
                    span.in_scope(|| error!("can't publish CloseInput event"));
                    // don't exit, continue to close the port
                }

                if let Err(err) = port_events.try_send(ZmqInputPortEvent::Closed) {
                    #[cfg(feature = "tracing")]
                    span.in_scope(|| warn!("did not send InputPortEvent::Closed: {}", err));
                }

                *input_state = ZmqInputPortState::Closed;

                let topics = input_topics(input_port_id);
                if unsubscribe_topics(&topics, sub_queue).await.is_err() {
                    #[cfg(feature = "tracing")]
                    span.in_scope(|| error!("topic unsubscription failed"));
                }

                if response_chan.send(Ok(())).await.is_err() {
                    #[cfg(feature = "tracing")]
                    span.in_scope(|| warn!("response channel is closed"));
                }
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
