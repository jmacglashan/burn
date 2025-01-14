use super::{
    execution::{ExecutionMode, Processor},
    store::OptimizationStore,
    Ops, Stream, TensorOpsDescription,
};
use crate::{FusionBackend, HandleContainer};

/// Keep track of multiple concurrent streams of operations.
///
/// TODO: Actually support multiple streams.
pub struct MultiStream<B: FusionBackend> {
    items: Vec<Item<B>>,
    optimizations: OptimizationStore<B::Optimization>,
}

struct Item<B: FusionBackend> {
    stream: Stream<B>,
    executor: Processor<B>,
}

impl<B: FusionBackend> MultiStream<B> {
    pub(crate) fn new(device: B::FusionDevice) -> Self {
        Self {
            items: vec![Item::new(device)],
            optimizations: OptimizationStore::new(),
        }
    }

    /// Register a new tensor operation.
    pub fn register(
        &mut self,
        ops_desc: TensorOpsDescription,
        ops: Box<dyn Ops<B>>,
        handles: &mut HandleContainer<B>,
    ) {
        // TODO: Support more than only one stream.
        if let Some(item) = self.items.first_mut() {
            item.stream.add(ops_desc, ops);
            item.executor.process(
                &mut item.stream,
                &mut self.optimizations,
                handles,
                ExecutionMode::Lazy,
            );
        };
    }

    /// Drain the streams.
    pub fn drain(&mut self, handles: &mut HandleContainer<B>) {
        self.items.iter_mut().for_each(|item| {
            item.executor.process(
                &mut item.stream,
                &mut self.optimizations,
                handles,
                ExecutionMode::Sync,
            );
        });
    }
}

impl<B: FusionBackend> Item<B> {
    fn new(device: B::FusionDevice) -> Self {
        Self {
            executor: Processor::new(B::optimizations(device.into())),
            stream: Stream::new(),
        }
    }
}
