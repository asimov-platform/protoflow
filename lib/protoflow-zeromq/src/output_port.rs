// This is free and unencumbered software released into the public domain.

use crate::{subscribe_topics, unsubscribe_topics, ZmqTransport, ZmqTransportEvent};
use protoflow_core::{
    prelude::{fmt, format, vec, Bytes, String, ToString, Vec},
    InputPortID, OutputPortID, PortError, PortState,
};
use tokio::sync::{
    mpsc::{channel, Sender},
    RwLock,
};

#[cfg(feature = "tracing")]
use tracing::{debug, error, info, trace, trace_span, warn};

#[derive(Debug, Clone)]
pub enum ZmqOutputPortRequest {
    Close,
    Send(Bytes),
}

#[derive(Debug, Clone)]
pub enum ZmqOutputPortState {
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

impl fmt::Display for ZmqOutputPortState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ZmqOutputPortState::*;
        match *self {
            Open(..) => write!(f, "Open"),
            Connected(.., ref id) => {
                write!(f, "Connected({:?})", isize::from(*id),)
            }
            Closed => write!(f, "Closed"),
        }
    }
}

impl ZmqOutputPortState {
    pub fn state(&self) -> PortState {
        use ZmqOutputPortState::*;
        match self {
            Open(..) => PortState::Open,
            Connected(..) => PortState::Connected,
            Closed => PortState::Closed,
        }
    }
}

fn output_topics(source: OutputPortID, target: InputPortID) -> Vec<String> {
    vec![
        format!("{}:ackConn:{}", target, source),
        format!("{}:ackMsg:{}:", target, source),
        format!("{}:closeIn", target),
    ]
}

pub fn start_output_worker(
    transport: &ZmqTransport,
    output_port_id: OutputPortID,
) -> Result<(), PortError> {
    #[cfg(feature = "tracing")]
    let span = trace_span!("ZmqTransport::output_port_worker", ?output_port_id);

    let (conn_send, mut conn_recv) = channel(1);
    let (to_worker_send, mut to_worker_recv) = channel(1);

    {
        let mut outputs = transport.tokio.block_on(transport.outputs.write());
        if outputs.contains_key(&output_port_id) {
            return Err(PortError::Invalid(output_port_id.into()));
        }
        let state = ZmqOutputPortState::Open(conn_send, to_worker_send.clone());
        #[cfg(feature = "tracing")]
        span.in_scope(|| trace!("saving new state: {}", state));
        outputs.insert(output_port_id, RwLock::new(state));
    }

    let sub_queue = transport.sub_queue.clone();
    let pub_queue = transport.pub_queue.clone();
    let outputs = transport.outputs.clone();

    #[cfg(feature = "tracing")]
    span.in_scope(|| trace!("spawning"));

    tokio::task::spawn(async move {
        let Some((input_port_id, conn_confirm)) = conn_recv.recv().await else {
            // all senders have dropped, i.e. there's no connection request coming
            #[cfg(feature = "tracing")]
            debug!(parent: &span, "no connection request");
            return;
        };

        #[cfg(feature = "tracing")]
        let span = trace_span!(parent: &span, "task", ?input_port_id);

        let topics = output_topics(output_port_id, input_port_id);
        if subscribe_topics(&topics, &sub_queue).await.is_err() {
            #[cfg(feature = "tracing")]
            span.in_scope(|| error!("topic subscription failed"));
            return;
        }

        let (msg_req_send, mut msg_req_recv) = channel(1);

        // Output worker loop:
        //   1. Send connection attempt
        //   2. Send messages
        //     2.1 Wait for ACK
        //     2.2. Resend on timeout
        //   3. Send disconnect events

        loop {
            #[cfg(feature = "tracing")]
            let span = trace_span!(parent: &span, "connect_loop");

            #[cfg(feature = "tracing")]
            span.in_scope(|| trace!("sending connection attempt..."));

            if pub_queue
                .send(ZmqTransportEvent::Connect(output_port_id, input_port_id))
                .await
                .is_err()
            {
                #[cfg(feature = "tracing")]
                span.in_scope(|| error!("publish channel is closed"));
                return;
            }

            let Some(response) = to_worker_recv.recv().await else {
                #[cfg(feature = "tracing")]
                span.in_scope(|| error!("all senders to worker have dropped?"));
                return;
            };

            #[cfg(feature = "tracing")]
            span.in_scope(|| trace!(?response, "got response"));

            use ZmqTransportEvent::*;
            match response {
                AckConnection(_, input_port_id) => {
                    let response = match outputs.read().await.get(&output_port_id) {
                        None => {
                            #[cfg(feature = "tracing")]
                            span.in_scope(|| error!("port state not found"));
                            Err(PortError::Invalid(output_port_id.into()))
                        }
                        Some(output_state) => {
                            let mut output_state = output_state.write().await;
                            debug_assert!(matches!(*output_state, ZmqOutputPortState::Open(..)));
                            *output_state = ZmqOutputPortState::Connected(
                                msg_req_send,
                                to_worker_send,
                                input_port_id,
                            );

                            #[cfg(feature = "tracing")]
                            span.in_scope(|| info!("Connected!"));

                            Ok(())
                        }
                    };

                    if conn_confirm.send(response).await.is_err() {
                        #[cfg(feature = "tracing")]
                        span.in_scope(|| warn!("connection confirmation channel is closed"));
                        // don't exit, proceed to send loop
                    }
                    drop(conn_confirm);

                    break;
                }
                _ => continue,
            }
        }

        let mut seq_id = 1;
        'send: while let Some((request, response_chan)) = msg_req_recv.recv().await {
            #[cfg(feature = "tracing")]
            let span = trace_span!(parent: &span, "send_loop", ?seq_id);

            #[cfg(feature = "tracing")]
            span.in_scope(|| trace!(?request, "sending request"));

            let respond = |response| async {
                if response_chan.send(response).await.is_err() {
                    #[cfg(feature = "tracing")]
                    span.in_scope(|| warn!("response channel is closed"));
                }
            };

            match request {
                ZmqOutputPortRequest::Close => {
                    let response = pub_queue
                        .send(ZmqTransportEvent::CloseOutput(
                            output_port_id,
                            input_port_id,
                        ))
                        .await
                        .map_err(|e| PortError::Other(e.to_string()));
                    respond(response).await;
                    break 'send;
                }
                ZmqOutputPortRequest::Send(bytes) => {
                    if pub_queue
                        .send(ZmqTransportEvent::Message(
                            output_port_id,
                            input_port_id,
                            seq_id,
                            bytes,
                        ))
                        .await
                        .is_err()
                    {
                        respond(Err(PortError::SendFailed)).await;
                        continue 'send;
                    }

                    'recv: loop {
                        let Some(event) = to_worker_recv.recv().await else {
                            #[cfg(feature = "tracing")]
                            span.in_scope(|| error!("all senders to worker have dropped"));

                            respond(Err(PortError::Invalid(output_port_id.into()))).await;
                            break 'send;
                        };

                        #[cfg(feature = "tracing")]
                        span.in_scope(|| trace!(?event, "received event"));

                        use ZmqTransportEvent::*;
                        match event {
                            AckMessage(_, _, ack_id) => {
                                if ack_id == seq_id {
                                    #[cfg(feature = "tracing")]
                                    span.in_scope(|| trace!(?ack_id, "msg-ack matches"));
                                    respond(Ok(())).await;
                                    break 'recv;
                                } else {
                                    #[cfg(feature = "tracing")]
                                    span.in_scope(|| {
                                        trace!(?ack_id, "got msg-ack for different sequence")
                                    });
                                }
                            }
                            CloseInput(_) => {
                                // report that the input port was closed
                                respond(Err(PortError::Disconnected)).await;
                                break 'send;
                            }

                            // ignore others, we shouldn't receive any new conn-acks
                            // nor should we be receiving input port events
                            AckConnection(..) | Connect(..) | Message(..) | CloseOutput(..) => {
                                continue 'recv
                            }
                        }
                    }
                }
            }

            seq_id += 1;
        }

        let outputs = outputs.read().await;
        let Some(output_state) = outputs.get(&output_port_id) else {
            #[cfg(feature = "tracing")]
            span.in_scope(|| error!("port state not found"));
            return;
        };
        let mut output_state = output_state.write().await;
        debug_assert!(matches!(*output_state, ZmqOutputPortState::Connected(..)));
        *output_state = ZmqOutputPortState::Closed;

        if unsubscribe_topics(&topics, &sub_queue).await.is_err() {
            #[cfg(feature = "tracing")]
            span.in_scope(|| error!("topic unsubscription failed"));
        }
    });

    Ok(())
}
