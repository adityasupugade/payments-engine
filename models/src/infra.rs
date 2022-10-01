use std::sync::Arc;

use tokio::runtime::Runtime;

pub fn init_runtime(worker_threads: usize, blocking_threads: usize) -> Result<Arc<Runtime>, std::io::Error> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(worker_threads)
        .max_blocking_threads(blocking_threads)
        .thread_name("syncer-rt")
        .build()?;

    Ok(Arc::new(rt))
}