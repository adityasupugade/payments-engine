use core::error::Error;
use std::sync::Arc;

use mem_store::mem_store::MemStore;
use tokio::runtime::Runtime;
use tokio_stream::StreamExt;
use csv::reader::{Reader, read_csv};


pub async fn process_transactions(reader: &mut Reader, store: &mut MemStore, rt: Arc<Runtime>) -> Result<(), Error> {

    let mut rdr = read_csv(reader).await;

    while let Some(transaction) = rdr.next().await {
        println!("{:?}", transaction);
    }

    Ok(())
}