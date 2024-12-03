// This is free and unencumbered software released into the public domain.

#![no_std]
#![deny(unsafe_code)]

#[doc(hidden)]
pub use protoflow_core::prelude;

#[path = "protoflow.zmq.rs"]
mod protoflow_zmq;

mod input_port;
use input_port::*;

mod output_port;
use output_port::*;

mod socket;
use socket::*;

mod event;
use event::*;

extern crate std;

use protoflow_core::{
    prelude::{Arc, BTreeMap, Bytes, ToString},
    InputPortID, OutputPortID, PortError, PortResult, PortState, Transport,
};

use tokio::sync::{
    mpsc::{channel, error::TryRecvError, Sender},
    RwLock,
};
use zeromq::{util::PeerIdentity, Socket, SocketOptions};

#[cfg(feature = "tracing")]
use tracing::trace;

const DEFAULT_PUB_SOCKET: &str = "tcp://127.0.0.1:10000";
const DEFAULT_SUB_SOCKET: &str = "tcp://127.0.0.1:10001";

pub struct ZmqTransport {
    tokio: tokio::runtime::Handle,

    pub_queue: Sender<ZmqTransportEvent>,
    sub_queue: Sender<ZmqSubscriptionRequest>,

    outputs: Arc<RwLock<BTreeMap<OutputPortID, RwLock<ZmqOutputPortState>>>>,
    inputs: Arc<RwLock<BTreeMap<InputPortID, RwLock<ZmqInputPortState>>>>,
}

impl Default for ZmqTransport {
    fn default() -> Self {
        Self::new(DEFAULT_PUB_SOCKET, DEFAULT_SUB_SOCKET)
    }
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

        let (pub_queue, pub_queue_recv) = channel(1);

        let (sub_queue, sub_queue_recv) = channel(1);

        let transport = Self {
            pub_queue,
            sub_queue,
            tokio,
            outputs,
            inputs,
        };

        start_pub_socket_worker(psock, pub_queue_recv);
        start_sub_socket_worker(&transport, ssock, sub_queue_recv);

        transport
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

        start_input_worker(self, new_id).map(|_| new_id)
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

        start_output_worker(self, new_id).map(|_| new_id)
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

            let (close_send, mut close_recv) = channel(1);

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

            let (close_send, mut close_recv) = channel(1);

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

            let (confirm_send, mut confirm_recv) = channel(1);

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

            let (ack_send, mut ack_recv) = channel(1);

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
