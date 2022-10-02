use std::sync::Arc;

use futures::Future;
use tokio::{runtime::Runtime, task::JoinHandle};
use tracing::{Span, Instrument};

#[derive(Clone)]
pub struct SpannedRuntime {
    runtime:    Arc<Runtime>,
    span:   Span,
}

impl SpannedRuntime {
    fn new(runtime: Arc<Runtime>, span: tracing::Span) -> Self {
        Self {runtime, span}
    }

    pub fn span(&self) -> Span {
        self.span.clone()
    }

    pub fn spawn<T>(&self, future: T) -> JoinHandle<T::Output> 
            where T: Future+Send+'static,
            T::Output: Send+'static {
        let sp = self.span.clone();
        self.runtime.spawn(async move {future.instrument(sp).await} )
    }

    pub fn spawn_blocking<F, R>(&self, func: F) -> JoinHandle<R>
        where
            F: FnOnce() -> R + Send + 'static,
            R: Send + 'static, {
        let sp = self.span.clone();
        self.runtime.spawn_blocking(sp.in_scope(||{func}))
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        let sp = self.span.clone();
        self.runtime.block_on(async move { future.instrument(sp).await })
    }
}

pub fn get_runtime(worker_threads: usize, blocking_threads: usize, span: tracing::Span) -> Result<SpannedRuntime, std::io::Error> {
    let rt = init_runtime(worker_threads, blocking_threads)?;
    Ok(SpannedRuntime::new(rt, span))
}
pub fn init_runtime(worker_threads: usize, blocking_threads: usize) -> Result<Arc<Runtime>, std::io::Error> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(worker_threads)
        .max_blocking_threads(blocking_threads)
        .thread_name("syncer-rt")
        .build()?;

    Ok(Arc::new(rt))
}