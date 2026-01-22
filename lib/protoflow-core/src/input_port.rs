// This is free and unencumbered software released into the public domain.

use crate::{
    prelude::{fmt, Arc, Cow, MaybeLabeled, MaybeNamed, PhantomData, RwLock},
    InputPortID, Message, MessageReceiver, Port, PortError, PortID, PortResult, PortState, System,
    Transport,
};

#[derive(Clone)] //, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct InputPort<T: Message> {
    pub(crate) state: Arc<RwLock<InputPortState>>,
    _phantom: PhantomData<T>,
}

impl<T: Message> InputPort<T> {
    pub fn new<X: Transport + Default>(system: &System<X>) -> Self {
        let id = system.connection_config.borrow_mut().add_input();
        let connection = Default::default();
        let state = Arc::new(RwLock::new(InputPortState { id, connection }));
        Self {
            _phantom: PhantomData,
            state,
        }
    }

    pub fn close(&mut self) -> PortResult<bool> {
        let mut state = self.state.write();
        let InputPortConnection::Running(ref transport) = state.connection else {
            return Ok(false);
        };
        transport.close(PortID::Input(state.id))?;
        state.connection = InputPortConnection::Closed;
        Ok(true)
    }

    pub fn recv(&self) -> PortResult<Option<T>> {
        let state = self.state.read();
        let InputPortConnection::Running(ref transport) = state.connection else {
            return Err(PortError::Disconnected);
        };

        match transport.recv(state.id)? {
            None => Ok(None), // EOS (port closed)
            Some(encoded_message) => {
                if encoded_message.is_empty() {
                    Ok(None) // EOS (port disconnected)
                } else {
                    match T::decode_length_delimited(encoded_message) {
                        Ok(message) => Ok(Some(message)),
                        Err(err) => Err(err.into()),
                    }
                }
            }
        }
    }

    pub fn try_recv(&self) -> PortResult<Option<T>> {
        let state = self.state.read();
        let InputPortConnection::Running(ref transport) = state.connection else {
            return Err(PortError::Disconnected);
        };

        match transport.try_recv(state.id)? {
            None => Ok(None), // EOS
            Some(encoded_message) => match T::decode(encoded_message) {
                Ok(message) => Ok(Some(message)),
                Err(err) => Err(err.into()),
            },
        }
    }
}

impl<T: Message> MaybeNamed for InputPort<T> {
    fn name(&self) -> Option<Cow<str>> {
        None // TODO
    }
}

impl<T: Message> MaybeLabeled for InputPort<T> {
    fn label(&self) -> Option<Cow<str>> {
        None // TODO
    }
}

impl<T: Message> Port for InputPort<T> {
    fn id(&self) -> PortID {
        PortID::Input(self.state.read().id)
    }

    fn state(&self) -> PortState {
        let state = self.state.read();
        match state.connection {
            InputPortConnection::Closed => PortState::Closed,
            InputPortConnection::Ready => PortState::Open,
            InputPortConnection::Running(ref transport) => transport
                .state(PortID::Input(state.id))
                .unwrap_or(PortState::Closed),
        }
    }

    fn close(&mut self) -> PortResult<bool> {
        InputPort::close(self)
    }
}

impl<T: Message> MessageReceiver<T> for InputPort<T> {
    fn recv(&self) -> PortResult<Option<T>> {
        InputPort::recv(self)
    }

    fn try_recv(&self) -> PortResult<Option<T>> {
        InputPort::try_recv(self)
    }
}

impl<T: Message> fmt::Display for InputPort<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "→{}", self.id())
    }
}

impl<T: Message> fmt::Debug for InputPort<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("InputPort")
            .field("state", &self.state.read())
            .finish()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct InputPortState {
    pub(crate) id: InputPortID,
    pub(crate) connection: InputPortConnection,
}

#[derive(Clone, Default)]
pub(crate) enum InputPortConnection {
    #[default]
    Ready,
    Running(Arc<dyn Transport>),
    Closed,
}

impl core::fmt::Debug for InputPortConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InputPortConnection::{}",
            match self {
                Self::Ready => "Ready",
                Self::Running(_) => "Running",
                Self::Closed => "Closed",
            }
        )
    }
}
