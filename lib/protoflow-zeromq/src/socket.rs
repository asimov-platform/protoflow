// This is free and unencumbered software released into the public domain.

use crate::{ZmqInputPortState, ZmqOutputPortState, ZmqTransport, ZmqTransportEvent};
use core::fmt::Error;
use protoflow_core::{
    prelude::{BTreeMap, String, Vec},
    InputPortID, OutputPortID,
};
use tokio::sync::{
    mpsc::{error::SendError, Receiver, Sender},
    RwLock,
};
use zeromq::{SocketRecv, SocketSend, ZmqMessage};

#[derive(Clone, Debug)]
pub enum ZmqSubscriptionRequest {
    Subscribe(String),
    Unsubscribe(String),
}

#[cfg(feature = "tracing")]
use tracing::trace;

pub fn start_pub_socket_worker(psock: zeromq::PubSocket, pub_queue: Receiver<ZmqTransportEvent>) {
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

pub async fn subscribe_topics(
    topics: &[String],
    sub_queue: &Sender<ZmqSubscriptionRequest>,
) -> Result<(), SendError<ZmqSubscriptionRequest>> {
    let mut handles = Vec::with_capacity(topics.len());
    for topic in topics {
        handles.push(sub_queue.send(ZmqSubscriptionRequest::Subscribe(topic.clone())));
    }
    for handle in handles {
        handle.await?;
    }
    Ok(())
}

pub async fn unsubscribe_topics(
    topics: &[String],
    sub_queue: &Sender<ZmqSubscriptionRequest>,
) -> Result<(), SendError<ZmqSubscriptionRequest>> {
    let mut handles = Vec::with_capacity(topics.len());
    for topic in topics {
        handles.push(sub_queue.send(ZmqSubscriptionRequest::Unsubscribe(topic.clone())));
    }
    for handle in handles {
        handle.await?;
    }
    Ok(())
}

pub fn start_sub_socket_worker(
    transport: &ZmqTransport,
    ssock: zeromq::SubSocket,
    sub_queue: Receiver<ZmqSubscriptionRequest>,
) {
    let outputs = transport.outputs.clone();
    let inputs = transport.inputs.clone();
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
