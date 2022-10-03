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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::fmt::Error;
    use futures::{FutureExt, TryStreamExt};
    use models::{logger::create_span, transactions::{Transaction, TransactionKind}};
    use tokio_stream::StreamExt;

    use super::read_csv;


    #[test]
    fn test_read_csv() {
        let rt = Arc::new(models::infra::get_runtime(1, 1, create_span()).unwrap());
        rt.block_on(run_read_csv_test())
    }

    async fn run_read_csv_test() {
        let mut input = r"
        type,client,tx,amount
        deposit,1,1,2
        deposito,1,2,2.0
        withdrawal,1,3,5.0
        resolve,1,4,
        resolve,1,5, 50.0
        dispute,1,6,
        dispute,1,7, 50.0
        chargeback,1,8,
        chargeback,1,9, 100.000
        deposit,1,10,8.4521
        withdrawal,1,11,7.9462
        withdrawal,1,12,
        deposit,1,12,"
            .as_bytes();

        let result = read_csv(&mut input)
            .map(|tx| tx.map_err(|_| Error))
            .await
            .collect::<Vec<_>>()
            .await;

        let expected = vec![
            Ok(Transaction { kind: TransactionKind::Deposit, client_id: 1, id: 1, amount: Some(2.0), under_dispute: false }),
            Err(Error),
            Ok(Transaction { kind: TransactionKind::Withdrawal, client_id: 1, id: 3, amount: Some(5.0), under_dispute: false }),
            Ok(Transaction { kind: TransactionKind::Resolve, client_id: 1, id: 4, amount: None, under_dispute: false }),
            Ok(Transaction { kind: TransactionKind::Resolve, client_id: 1, id: 5, amount: Some(50.0), under_dispute: false }),
            Ok(Transaction { kind: TransactionKind::Dispute, client_id: 1, id: 6, amount: None, under_dispute: false }),
            Ok(Transaction { kind: TransactionKind::Dispute, client_id: 1, id: 7, amount: Some(50.0), under_dispute: false }),
            Ok(Transaction { kind: TransactionKind::ChargeBack, client_id: 1, id: 8, amount: None, under_dispute: false }), 
            Ok(Transaction { kind: TransactionKind::ChargeBack, client_id: 1, id: 9, amount: Some(100.0), under_dispute: false }), 
            Ok(Transaction { kind: TransactionKind::Deposit, client_id: 1, id: 10, amount: Some(8.4521), under_dispute: false }), 
            Ok(Transaction { kind: TransactionKind::Withdrawal, client_id: 1, id: 11, amount: Some(7.9462), under_dispute: false }), 
            Ok(Transaction { kind: TransactionKind::Withdrawal, client_id: 1, id: 12, amount: None, under_dispute: false }), 
            Ok(Transaction { kind: TransactionKind::Deposit, client_id: 1, id: 12, amount: None, under_dispute: false })
        ];

        assert_eq!(result, expected)
    }
}