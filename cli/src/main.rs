mod process;

use std::{env, path::PathBuf, sync::Arc};
use mem_store::mem_store::MemStore;
use core::error::Error;
use tokio::{fs::File, runtime::Runtime};
use crate::process::process_transactions;

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
    
    let rt = core::infra::init_runtime(2, 1)?;
    let rtc = rt.clone();
    let p = PathBuf::from("/home/aditya/transactions.csv");
    rt.block_on(init(p, rtc))?;
    Ok(())
}

async fn init(path: PathBuf, rt: Arc<Runtime>) -> Result<(), Error> {
    let mut file = File::open(path).await?;
    let mut store = MemStore::default();
    process_transactions(&mut file, &mut store, rt).await?;
    Ok(())
}