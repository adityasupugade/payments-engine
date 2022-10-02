use models::{error::Error, infra::SpannedRuntime};
use std::sync::Arc;

use mem_store::mem_store::MemStore;
use publish::publish::Publisher;
use tokio_stream::StreamExt;
use csv::{reader::{Reader, read_csv}, writer::{write_csv, Writer}};


pub async fn process_transactions(reader: &mut Reader, store: MemStore, writer: &mut Writer, rt: Arc<SpannedRuntime>, worker_count: u16) -> Result<(), Error> {

    let mut rdr = read_csv(reader).await;
    let mut publisher = Publisher::new(store, rt, worker_count);
    while let Some(t) = rdr.next().await {
        publisher.post_txn(t.unwrap()).await?;
    }
    publisher.shutdown_gracefully().await;
    let report = publisher.get_report().await?;
    write_csv(writer, report).await?;
    Ok(())
}