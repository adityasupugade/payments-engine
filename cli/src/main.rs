mod process;

use std::{env, path::PathBuf, sync::Arc, str::FromStr};
use mem_store::mem_store::MemStore;
use models::{error::Error, logger::{self, create_span}, infra::SpannedRuntime};
use tokio::fs::File;
use crate::process::process_transactions;

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    let input_path = &args[1];
    let log_file_dir = PathBuf::from_str("./log").unwrap();
    let filter = "debug".to_string();
    let logger = logger::Logger::new(log_file_dir, filter);
    logger.start();

    let rt = Arc::new(models::infra::get_runtime(1, 1, create_span()).unwrap());
    let rtc = rt.clone();
    let path = PathBuf::from(input_path);
    rt.block_on(init(path, rtc))?;
    Ok(())
}

async fn init(path: PathBuf, rt: Arc<SpannedRuntime>) -> Result<(), Error> {
    let mut file = File::open(path).await?;
    let mut writer = tokio::io::stdout();
    let store = MemStore::default();
    process_transactions(&mut file, store, &mut writer, rt, 2).await?;
    Ok(())
}