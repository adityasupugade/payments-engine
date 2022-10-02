mod process;

use std::{env, path::PathBuf, sync::Arc, str::FromStr};
use mem_store::mem_store::MemStore;
use models::{error::Error, logger};
use tokio::{fs::File, runtime::Runtime};
use crate::process::process_transactions;

fn main() -> Result<(), Error> {
    let _args: Vec<String> = env::args().collect();

    let log_file_dir = PathBuf::from_str("./log").unwrap();
    let filter = "debug".to_string();
    let logger = logger::Logger::new(log_file_dir, filter);
    logger.start();

    let rt = models::infra::init_runtime(2, 1)?;
    let rtc = rt.clone();
    let p = PathBuf::from("/home/aditya_supugade/Homework/transactions.csv");
    rt.block_on(init(p, rtc))?;
    Ok(())
}

async fn init(path: PathBuf, rt: Arc<Runtime>) -> Result<(), Error> {
    let mut file = File::open(path).await?;
    let mut writer = tokio::io::stdout();
    let store = MemStore::default();
    process_transactions(&mut file, store, &mut writer, rt, 2).await?;
    Ok(())
}