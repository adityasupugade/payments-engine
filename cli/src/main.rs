mod process;

use std::{env, path::PathBuf, sync::Arc};
use mem_store::mem_store::MemStore;
use models::error::Error;
use tokio::{fs::File, runtime::Runtime};
use crate::process::process_transactions;

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    // println!("{:?}", args);
    
    let rt = models::infra::init_runtime(2, 1)?;
    let rtc = rt.clone();
    let p = PathBuf::from("/home/aditya_supugade/Homework/transactions.csv");
    rt.block_on(init(p, rtc))?;
    Ok(())
}

async fn init(path: PathBuf, rt: Arc<Runtime>) -> Result<(), Error> {
    let mut file = File::open(path).await?;
    let mut writer = tokio::io::stdout();
    let mut store = MemStore::default();
    process_transactions(&mut file, store, &mut writer, rt).await?;
    Ok(())
}