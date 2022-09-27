use anyhow::Error;
use tokio_stream::StreamExt;
use csv::reader::{Reader, read_csv};


pub async fn process_transactions(reader: &mut Reader) -> Result<(), Error> {

    let mut rdr = read_csv(reader).await;

    while let Some(transaction) = rdr.next().await {
        println!("{:?}", transaction);
    }

    Ok(())
}