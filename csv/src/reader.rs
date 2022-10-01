use models::transactions::Transaction;
use tokio_stream::StreamExt;

pub type Reader = dyn tokio::io::AsyncRead + Send + Sync + Unpin;

pub async fn read_csv(reader: &mut Reader) -> impl futures::Stream<Item = Result<Transaction, anyhow::Error>> + '_ {
    csv_async::AsyncReaderBuilder::new()
        .flexible(true)
        .trim(csv_async::Trim::All)
        .create_reader(reader)
        .into_records()
        .map(|record| {
            record
                .and_then(|r| {
                    r.deserialize::<Transaction>(None)
                        .map(std::convert::Into::into)
                })
                .map_err(anyhow::Error::from)
        })
}