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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use models::{logger::create_span, account::Account};
    use tokio::io::BufWriter;

    use crate::writer::write_csv;

    
    
    #[test]
    fn test_write_csv() {
        let rt = Arc::new(models::infra::get_runtime(1, 1, create_span()).unwrap());
        rt.block_on(run_write_csv_test())
    }

    async fn run_write_csv_test() {
        let input = vec![
            Account::load(1, 5.36, 1.58, false),
            Account::load(2, 8.19, 3.08, true),
        ];
        
        let account_stream = futures::stream::iter(input);
        let mut writer = BufWriter::new(Vec::<u8>::new());

        let result = write_csv(&mut writer, account_stream).await;

        assert!(result.is_ok());

        let buffer = writer.into_inner();
        let csv = String::from_utf8_lossy(&buffer);

        assert_eq!(
            csv,
            "client,available,held,total,locked\n1,5.36,1.58,6.94,false\n2,8.19,3.08,11.27,true\n"
        );
    }
}