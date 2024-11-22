// This is free and unencumbered software released into the public domain.

use crate::{
    prelude::{fmt, Arc, Bytes, Cow, MaybeLabeled, MaybeNamed, PhantomData, RwLock},
    Message, MessageSender, OutputPortID, Port, PortError, PortID, PortResult, PortState, System,
    Transport,
};

#[derive(Clone)] //, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OutputPort<T: Message> {
    pub(crate) state: Arc<RwLock<OutputPortState>>,
    _phantom: PhantomData<T>,
}

impl<T: Message> OutputPort<T> {
    pub fn new<X: Transport + Default>(system: &System<X>) -> Self {
        let id = system.connection_config.borrow_mut().add_output();
        let connection = Default::default();
        let state = Arc::new(RwLock::new(OutputPortState { id, connection }));
        Self {
            _phantom: PhantomData,
            state,
        }
    }

    pub fn close(&mut self) -> PortResult<bool> {
        let mut state = self.state.write();
        let OutputPortConnection::Running(ref transport) = state.connection else {
            return Ok(false);
        };
        transport.close(PortID::Output(state.id))?;
        state.connection = OutputPortConnection::Closed;
        Ok(true)
    }

    pub fn send<'a>(&self, message: impl Into<&'a T>) -> PortResult<()>
    where
        T: 'a,
    {
        let state = self.state.read();
        let OutputPortConnection::Running(ref transport) = state.connection else {
            return Err(PortError::Disconnected);
        };
        let message: &T = message.into();
        let bytes = Bytes::from(message.encode_length_delimited_to_vec());
        transport.send(state.id, bytes)
    }
}

impl<T: Message> MaybeNamed for OutputPort<T> {
    fn name(&self) -> Option<Cow<str>> {
        None // TODO
    }
}

impl<T: Message> MaybeLabeled for OutputPort<T> {
    fn label(&self) -> Option<Cow<str>> {
        None // TODO
    }
}

impl<T: Message> Port for OutputPort<T> {
    fn id(&self) -> PortID {
        PortID::Output(self.state.read().id)
    }

    fn state(&self) -> PortState {
        let state = self.state.read();
        match state.connection {
            OutputPortConnection::Closed => PortState::Closed,
            OutputPortConnection::Ready => PortState::Open,
            OutputPortConnection::Running(ref transport) => transport
                .state(PortID::Output(state.id))
                .unwrap_or(PortState::Closed),
        }
    }

    fn close(&mut self) -> PortResult<bool> {
        OutputPort::close(self)
    }
}

impl<T: Message> MessageSender<T> for OutputPort<T> {
    fn send<'a>(&self, message: impl Into<&'a T>) -> PortResult<()>
    where
        T: 'a,
    {
        OutputPort::send(self, message)
    }
}

impl<T: Message> fmt::Display for OutputPort<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}→", self.id())
    }
}

impl<T: Message> fmt::Debug for OutputPort<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("OutputPort")
            .field("state", &self.state.read())
            .finish()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct OutputPortState {
    pub(crate) id: OutputPortID,
    pub(crate) connection: OutputPortConnection,
}

#[derive(Clone, Default)]
pub(crate) enum OutputPortConnection {
    #[default]
    Ready,
    Running(Arc<dyn Transport>),
    Closed,
}

impl core::fmt::Debug for OutputPortConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "OutputPortConnection::{}",
            match self {
                Self::Ready => "Ready",
                Self::Running(_) => "Running",
                Self::Closed => "Closed",
            }
        )
    }
}
