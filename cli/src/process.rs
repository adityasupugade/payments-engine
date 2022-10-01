use models::error::Error;
use std::sync::Arc;

use mem_store::mem_store::MemStore;
use publish::publish::Publisher;
use tokio::runtime::Runtime;
use tokio_stream::StreamExt;
use csv::reader::{Reader, read_csv};


pub async fn process_transactions(reader: &mut Reader, store: MemStore, rt: Arc<Runtime>) -> Result<(), Error> {

    let mut rdr = read_csv(reader).await;
    let mut a = Publisher::new(store, rt, 2);
    while let Some(t) = rdr.next().await {
        println!("{:?}", t);
        a.post_txn(t.unwrap()).await?;
    }
    a.shutdown_gracefully().await;
    // print!("{:?}", store);
    Ok(())
}