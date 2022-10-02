use futures::StreamExt;
use models::{account::Account, error::Error};

pub type Writer = dyn tokio::io::AsyncWrite + Send + Sync + Unpin;

pub async fn write_csv(writer: &mut Writer, mut account_stream: impl futures::Stream<Item = Account> + Send + Unpin) -> Result<(), Error> {
    let mut writer = csv_async::AsyncSerializer::from_writer(writer);

    while let Some(mut account) = account_stream.next().await {
        account.to_max_display_precision();
        writer.serialize(account).await?;
    }

    Ok(())
}