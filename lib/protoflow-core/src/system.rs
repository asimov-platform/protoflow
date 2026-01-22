// This is free and unencumbered software released into the public domain.

use crate::{
    prelude::{
        fmt, Arc, BTreeMap, BTreeSet, Box, Bytes, PhantomData, Rc, RefCell, RwLock, String,
        ToString, VecDeque,
    },
    runtimes::StdRuntime,
    transports::MpscTransport,
    types::Any,
    Block, BlockError, BlockID, BlockResult, BoxedBlock, BoxedBlockType, InputPort,
    InputPortConnection, InputPortID, InputPortState, Message, OutputPort, OutputPortConnection,
    OutputPortID, OutputPortState, Port, PortID, PortResult, Process, Runtime, Transport,
};

#[cfg(feature = "tokio")]
use crate::{AsyncBlock, BoxedAsyncBlock};

#[cfg(feature = "tokio")]
pub type RuntimeHandle = tokio::runtime::Handle;

pub trait SystemBuilding {
    fn input_any(&self) -> InputPort<Any> {
        self.input()
    }

    fn input_bytes(&self) -> InputPort<Bytes> {
        self.input()
    }

    fn input_string(&self) -> InputPort<String> {
        self.input()
    }

    /// Creates a new input port inside the system.
    fn input<M: Message + 'static>(&self) -> InputPort<M>;

    fn output_any(&self) -> OutputPort<Any> {
        self.output()
    }

    fn output_bytes(&self) -> OutputPort<Bytes> {
        self.output()
    }

    fn output_string(&self) -> OutputPort<String> {
        self.output()
    }

    /// Creates a new output port inside the system.
    fn output<M: Message + 'static>(&self) -> OutputPort<M>;

    /// Instantiates a block inside the system.
    fn block<B: Block + Clone + 'static>(&mut self, block: B) -> B;

    ///
    #[cfg(feature = "tokio")]
    fn block_async<B: AsyncBlock + Clone + 'static>(&mut self, block: B) -> B;

    /// Connects two ports of two blocks in the system.
    ///
    /// Both ports must be of the same message type.
    fn connect<M: Message>(&mut self, source: &OutputPort<M>, target: &InputPort<M>) -> bool;

    /// Validates system for execution.
    fn validate(&self) -> BlockResult<()>;
}

pub trait SystemExecution {
    /// Prepare:
    ///  - Calls the transport layer to connect all the output->input ports.
    ///    The connections are defined by `SystemBuilding.connect()`.
    fn prepare(&self) -> BlockResult<()>;
    /// Executes the system, returning the system process.
    fn execute(self) -> BlockResult<Rc<dyn Process>>;
}

/// A system is a collection of blocks that are connected together.
pub struct System<X: Transport + Default + 'static = MpscTransport> {
    pub(crate) runtime: Arc<StdRuntime<X>>,

    /// The registered blocks in the system.
    pub(crate) blocks: VecDeque<BoxedBlockType>,

    pub(crate) connection_config: RefCell<SystemConnections>,

    _phantom: PhantomData<X>,
}

#[derive(Default, Debug)]
pub(crate) struct SystemConnections {
    pub(crate) outputs: BTreeMap<OutputPortID, Arc<RwLock<OutputPortState>>>,
    pub(crate) inputs: BTreeMap<InputPortID, Arc<RwLock<InputPortState>>>,
    pub(crate) connections: BTreeSet<(OutputPortID, InputPortID)>,
}

impl SystemConnections {
    pub(crate) fn add_output(&mut self) -> OutputPortID {
        let id = self.outputs.len() + 1;
        OutputPortID::try_from(id as isize).unwrap()
    }

    pub(crate) fn add_input(&mut self) -> InputPortID {
        let id = self.inputs.len() + 1;
        InputPortID::try_from(-(id as isize)).unwrap()
    }
}

pub type Subsystem<X> = System<X>;

impl fmt::Debug for System {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("System")
            .field("blocks", &self.blocks)
            .finish()
    }
}

impl<X: Transport + Default + 'static> System<X> {
    /// Builds a new system.
    pub fn build<F: FnOnce(&mut System<X>)>(f: F) -> Self {
        let transport = X::default();
        let runtime = StdRuntime::new(transport).unwrap();
        let mut system = System::new(&runtime);
        f(&mut system);
        system
    }

    /// Instantiates a new system.
    pub fn new(runtime: &Arc<StdRuntime<X>>) -> Self {
        Self {
            runtime: runtime.clone(),
            blocks: VecDeque::new(),
            connection_config: Default::default(),
            _phantom: PhantomData,
        }
    }

    pub fn execute(self) -> BlockResult<Rc<dyn Process>> {
        let mut runtime = self.runtime.clone();
        runtime.execute(self)
    }

    pub fn input<M: Message + 'static>(&self) -> InputPort<M> {
        let port = InputPort::new(self);
        let state = port.state.clone();
        let id = state.read().id;
        self.connection_config.borrow_mut().inputs.insert(id, state);
        port
    }

    pub fn output<M: Message + 'static>(&self) -> OutputPort<M> {
        let port = OutputPort::new(self);
        let state = port.state.clone();
        let id = state.read().id;
        self.connection_config
            .borrow_mut()
            .outputs
            .insert(id, state);
        port
    }

    pub fn block<B: Block + Clone + 'static>(&mut self, block: B) -> B {
        self.add_block(Box::new(block.clone()));
        block
    }

    #[cfg(feature = "tokio")]
    pub fn block_async<B: AsyncBlock + Clone + 'static>(&mut self, block: B) -> B {
        self.add_block_async(Box::new(block.clone()));
        block
    }

    #[doc(hidden)]
    pub fn add_block(&mut self, block: BoxedBlock) -> BlockID {
        let block_id = BlockID::from(self.blocks.len());
        self.blocks.push_back(BoxedBlockType::Normal(block));
        block_id
    }

    #[doc(hidden)]
    #[cfg(feature = "tokio")]
    pub fn add_block_async(&mut self, block: BoxedAsyncBlock) -> BlockID {
        let block_id = BlockID::from(self.blocks.len());
        self.blocks.push_back(BoxedBlockType::Async(block));
        block_id
    }

    #[doc(hidden)]
    pub fn get_block(&self, block_id: BlockID) -> Option<&BoxedBlockType> {
        self.blocks.get(block_id.into())
    }

    pub fn connect<M: Message>(&self, source: &OutputPort<M>, target: &InputPort<M>) -> bool {
        self.connect_by_id(source.id(), target.id()).unwrap()
    }

    #[doc(hidden)]
    pub fn connect_by_id(&self, source_id: PortID, target_id: PortID) -> PortResult<bool> {
        self.connection_config.borrow_mut().connections.insert((
            OutputPortID(source_id.into()),
            InputPortID(target_id.into()),
        ));
        Ok(true)
    }
}

impl SystemBuilding for System {
    fn input<M: Message + 'static>(&self) -> InputPort<M> {
        System::input(self)
    }

    fn output<M: Message + 'static>(&self) -> OutputPort<M> {
        System::output(self)
    }

    fn block<B: Block + Clone + 'static>(&mut self, block: B) -> B {
        System::block(self, block)
    }

    #[cfg(feature = "tokio")]
    fn block_async<B: AsyncBlock + Clone + 'static>(&mut self, block: B) -> B {
        System::block_async(self, block)
    }

    fn connect<M: Message>(&mut self, source: &OutputPort<M>, target: &InputPort<M>) -> bool {
        System::connect(self, source, target)
    }

    fn validate(&self) -> BlockResult<()> {
        Ok(()) // TODO
    }
}

impl SystemExecution for System {
    fn prepare(&self) -> BlockResult<()> {
        // Prepare opens ports in the runtime's transport and connects them
        // according to `self.connection_config`.

        let connection_config = self.connection_config.borrow();

        // A map to go from the pre-created system port IDs to the actual transport port IDs.
        let mut output_port_system_to_transport_id = BTreeMap::new();

        // Open output ports in transport
        for (system_id, state) in connection_config.outputs.iter() {
            let transport_id = self
                .runtime
                .transport
                .open_output()
                .map_err(BlockError::PortError)?;

            output_port_system_to_transport_id.insert(system_id, transport_id);

            let mut state = state.write();
            // Update the port's state with the transport port ID.
            state.id = transport_id;
            // And give the port access to the transport.
            state.connection = OutputPortConnection::Running(self.runtime.transport.clone());
        }

        // A map to go from the pre-created system port IDs to the actual transport port IDs.
        let mut input_port_system_to_transport_id = BTreeMap::new();

        // Open input ports in transport.
        for (system_id, state) in connection_config.inputs.iter() {
            let transport_id = self
                .runtime
                .transport
                .open_input()
                .map_err(BlockError::PortError)?;

            input_port_system_to_transport_id.insert(system_id, transport_id);

            let mut state = state.write();
            // Update the port's state with the transport port ID.
            state.id = transport_id;
            // And give the port access to the transport.
            state.connection = InputPortConnection::Running(self.runtime.transport.clone());
        }

        // Connect all the ports.
        for (system_out_id, system_in_id) in connection_config.connections.clone() {
            let transport_out_id = output_port_system_to_transport_id.get(&system_out_id);
            let transport_in_id = input_port_system_to_transport_id.get(&system_in_id);

            let Some((&transport_out_id, &transport_in_id)) =
                Option::zip(transport_out_id, transport_in_id)
            else {
                // This is programmer error and the system will definitively not be ok for
                // execution.
                return Err(BlockError::Other(
                    "Failed to connect ports for execution".to_string(),
                ));
            };

            self.runtime
                .transport
                .connect(transport_out_id, transport_in_id)
                .map_err(BlockError::PortError)?;
        }

        Ok(())
    }

    fn execute(self) -> BlockResult<Rc<dyn Process>> {
        SystemExecution::prepare(&self)?;
        self.execute()
    }
}
