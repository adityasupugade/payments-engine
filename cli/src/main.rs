mod process;

use std::{env, path::PathBuf};
use anyhow::Error;
use tokio::fs::File;
use crate::process::process_transactions;

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
    
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .max_blocking_threads(1)
        .thread_name("syncer-rt")
        .build()?;

    let p = PathBuf::from("/home/aditya/transactions.csv");
    rt.block_on(init(p))?;
    Ok(())
}

async fn init(path: PathBuf) -> Result<(), Error> {
    let mut file = File::open(path).await?;
    process_transactions(&mut file).await?;
    Ok(())
}