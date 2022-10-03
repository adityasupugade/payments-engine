use models::{transactions::{Transaction, TransactionKind}, error::{Error, ErrorKind}, account::Account, store::Store, infra::SpannedRuntime};
use std::{sync::Arc, pin::Pin};

use tokio::sync::mpsc::Receiver;

#[derive(Clone)]

pub struct Engine<S: Store> {
    store: S,
}

impl <S: Store> Engine<S> 
where S: 'static+Send+Clone{
    pub fn new(store: S) -> Self {
        Engine{store}
    }

    pub async fn start(&self, rt: Arc<SpannedRuntime>, rx : Receiver<Transaction>) -> tokio::task::JoinHandle<()> {
        let e = self.clone();
        rt.spawn(async move { let _ = Engine::process_txn(&e, rx).await; })
    }

    pub async fn report(&self) -> Result<Pin<Box<dyn futures::Stream<Item = Account> + Send>>, Error> {
        self.store.get_all_accounts().await
    }

    async fn process_txn(&self, mut rx : Receiver<Transaction>) -> Result<(), Error> {
        while let Some(transaction) = rx.recv().await {
            tracing::info!("Payment engine processing transaction with id {}", transaction.id);
            if !transaction.is_valid_amount() {
                tracing::error!("Transaction with id {} has negative amount", transaction.id);
                continue;
            }

            if let Err(_) = self.store.add_transaction(transaction.clone()).await {
                tracing::error!("Failed to add transaction with id {}",transaction.id);
                continue;
            }

            let transaction_result: Result<(), Error> = async {
                let mut account = self.store.get_account(transaction.client_id).await?;

                if account.locked {
                    tracing::error!("Account locked for client id {} transaction id {}", transaction.client_id, transaction.id);
                    return Err(Error::new(ErrorKind::EngineError("Account locked".to_string())));
                }

                self.apply_transaction(&mut account, &transaction).await?;

                self.store.update_account(&account).await?;
                Ok(())
            }.await;

            match transaction_result {
                Ok(_) => {},
                Err(_) => {
                    // Rollback the stored transaction.
                    match transaction.kind {
                        TransactionKind::Deposit | TransactionKind::Withdrawal => {
                            // tracing::warn!("Rolling back transaction for tx {}", transaction.id);
                            if let Err(_) = self.store.delete_transaction(transaction.id).await {
                                tracing::error!("Failed to rollback transaction: {}", transaction.id);
                                // return Err(Error::new(ErrorKind::Unknown("abc".to_string())));
                            }

                        },
                        TransactionKind::Dispute => {
                            if let Err(_) = self.store.set_transaction_under_dispute(transaction.id, false).await {
                                tracing::error!("Failed to rollback transaction: {}", transaction.id);
                                // return Err(Error::new(ErrorKind::Unknown("abc".to_string())));
                            }
                        },
                        TransactionKind::Resolve | TransactionKind::ChargeBack => {
                            if let Err(_) = self.store.set_transaction_under_dispute(transaction.id, true).await {
                                tracing::error!("Failed to rollback transaction: {}", transaction.id);
                                // return Err(Error::new(ErrorKind::Unknown("abc".to_string())));
                            }
                        },
                    };    
                }
            }
        }
        Ok(())
    }

    pub async fn apply_transaction(&self, account: &mut Account, transaction: &Transaction) -> Result<(), Error> {
        match transaction.kind {
            TransactionKind::Deposit => { return self.deposit(account, &transaction.amount.unwrap()).await;},
            TransactionKind::Withdrawal => { return self.withdrawal(account, &transaction.amount.unwrap()).await;},
            TransactionKind::Dispute => { return self.dispute(account, &transaction).await;},
            TransactionKind::Resolve => { return self.resolve(account, &transaction).await;},
            TransactionKind::ChargeBack => { return self.chargeback(account, &transaction).await;},
        }
    }

    async fn deposit(&self, account: &mut Account, amount: &f32) -> Result<(), Error> {
        account.available += amount;
        account.total += amount;
        Ok(())
    }

    async fn withdrawal(&self, account: &mut Account, amount: &f32) -> Result<(), Error> {
        if account.available < *amount {
            tracing::error!(?account, "Insufficient available funds");
            return Err(Error::new(ErrorKind::InsufficientAvailableFunds));
        }
        account.available -= amount;
        account.total -= amount;
        Ok(())
    }

    async fn dispute(&self, account: &mut Account, info: &Transaction) -> Result<(), Error> {
        let ref_transaction: Result<Transaction, Error> = self.store.get_transaction(info.id).await;
        match ref_transaction {
            Err(e) => {
                match &*e.kind {
                    ErrorKind::StoreError(_) => {
                        tracing::info!("Ignoring dispute no reference found for transaction {}", info.id);
                        return Ok(())
                    },
                    _ => return Err(e),
                }
                
            },
            Ok(ref_tx) => {
                if ref_tx.kind == TransactionKind::Deposit {
                    if account.client != ref_tx.client_id || ref_tx.client_id != info.client_id {
                        tracing::error!(?account, "Wrong client_id in transaction: {}, expected: {}, got: {}", info.id, account.client, info.client_id);
                        return Err(Error::new(ErrorKind::WrongClientError(info.id, account.client, info.client_id)));
                    } else if ref_tx.under_dispute {
                        tracing::error!(?account, "Double dispute for tx {}", info.id);
                        return Err(Error::new(ErrorKind::DoubleDispute(info.id)));
                    } else if account.available < ref_tx.amount.unwrap() {
                        tracing::error!(?account, "Insufficient available funds");
                        return Err(Error::new(ErrorKind::InsufficientAvailableFunds));
                    }
                    account.available -= ref_tx.amount.unwrap();
                    account.held += ref_tx.amount.unwrap();
                    self.store.set_transaction_under_dispute(info.id, true).await?;
                } else {
                    tracing::error!("Reference transaction {} is not a Deposit", info.id);
                    return Err(Error::new(ErrorKind::WrongTransactionRef(info.id)));
                }

                Ok(())
            }
        }
    }

    async fn resolve(&self, account: &mut Account, info: &Transaction) -> Result<(), Error> {
        let ref_transaction: Result<Transaction, Error> = self.store.get_transaction(info.id).await;
        match ref_transaction {
            Err(e) => {
                match *e.kind {
                    ErrorKind::StoreError(_) => {
                        tracing::info!("Ignoring resolve no reference found for transaction {}", info.id);
                        Ok(())
                    },
                    _ => return Err(e),
                }
                
            },
            Ok(ref_tx) => {
                if ref_tx.kind == TransactionKind::Deposit {
                    if account.client != ref_tx.client_id || ref_tx.client_id != info.client_id {
                        tracing::error!(?account, "Wrong client_id in transaction: {}, expected: {}, got: {}", info.id, account.client, info.client_id);
                        return Err(Error::new(ErrorKind::WrongClientError(info.id, account.client, info.client_id)));
                    } else if account.held < ref_tx.amount.unwrap() {
                        tracing::error!(?account, "Insufficient available funds");
                        return Err(Error::new(ErrorKind::InsufficientAvailableFunds));
                    } else if !ref_tx.under_dispute {
                        tracing::info!("Ignoring resolve for transaction {}. Not under dispute", info.id);
                        return Ok(());
                    }
                    account.held -= ref_tx.amount.unwrap();
                    account.available += ref_tx.amount.unwrap();
                    self.store.set_transaction_under_dispute(info.id, false).await?;
                } else {
                    tracing::error!("Reference transaction {} is not a Deposit", info.id);
                    return Err(Error::new(ErrorKind::WrongTransactionRef(info.id)));
                }

                Ok(())
            }
        }
    }

    async fn chargeback(&self, account: &mut Account, info: &Transaction) -> Result<(), Error> {
        let ref_transaction: Result<Transaction, Error> = self.store.get_transaction(info.id).await;
        match ref_transaction {
            Err(e) => {
                match *e.kind {
                    ErrorKind::StoreError(_) => {
                        tracing::info!("Ignoring chargeback no reference found for transaction {}", info.id);
                        Ok(())
                    },
                    _ => return Err(e),
                }
                
            },
            Ok(ref_tx) => {
                if ref_tx.kind == TransactionKind::Deposit {
                    if account.client != ref_tx.client_id || ref_tx.client_id != info.client_id {
                        tracing::error!(?account, "Wrong client_id in transaction: {}, expected: {}, got: {}", info.id, account.client, info.client_id);
                        return Err(Error::new(ErrorKind::WrongClientError(info.id, account.client, info.client_id)));
                    } else if account.held < ref_tx.amount.unwrap() {
                        tracing::error!(?account, "Insufficient available funds");
                        return Err(Error::new(ErrorKind::InsufficientAvailableFunds));
                    } else if !ref_tx.under_dispute {
                        tracing::info!("Ignoring chargeback for transaction {}. Not under dispute", info.id);
                        return Ok(());
                    }
                    // if everything is fine: update the account
                    account.held -= ref_tx.amount.unwrap();
                    account.total -= ref_tx.amount.unwrap();
                    account.locked = true;
                    // set to not under dispute
                    self.store
                        .set_transaction_under_dispute(info.id, false)
                        .await?;
                } else {
                    tracing::error!("Reference transaction {} is not a Deposit", info.id);
                    return Err(Error::new(ErrorKind::WrongTransactionRef(info.id)));
                }

                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use mem_store::mem_store::MemStore;
    use models::{account::Account, store::Store, transactions::{Transaction, TransactionKind}, logger::create_span, infra::SpannedRuntime};

    use tracing_test::traced_test;
    use super::Engine;

    #[test]
    fn test_deposit() {
        let span = create_span();
        let rt = Arc::new(models::infra::get_runtime(1, 1, span).unwrap());
        let rtc = rt.clone();
        let account = Account::new(1);
        let store = MemStore::default();
        rt.block_on(run_deposit_test(account, store, rtc))
    }

    async fn run_deposit_test(account: Account, store: MemStore, rt: Arc<SpannedRuntime>) {
        store.update_account(&account).await.unwrap();
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let worker = Engine::new(store.clone()).start(rt.clone(), rx).await;
        tx.send(Transaction::new(TransactionKind::Deposit, 1, 1, Some(10.0))).await.unwrap();

        drop(tx);
        worker.await.unwrap();

        let account = store.get_account(account.client).await.unwrap();
        assert_eq!(account.available, 10.0);
        assert_eq!(account.held, 0.0);
        assert_eq!(account.total, 10.0);
    }

    #[test]
    fn test_withdrawal() {
        let span = create_span();
        let rt = Arc::new(models::infra::get_runtime(1, 1, span).unwrap());
        let rtc = rt.clone();
        let account = Account::load(1, 10.0, 0.0, false);
        let store = MemStore::default();
        rt.block_on(run_withdrawal_test(account, store, rtc))
    }

    async fn run_withdrawal_test(account: Account, store: MemStore, rt: Arc<SpannedRuntime>) {
        store.update_account(&account).await.unwrap();
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let worker = Engine::new(store.clone()).start(rt.clone(), rx).await;
        tx.send(Transaction::new(TransactionKind::Withdrawal, 1, 1, Some(5.0))).await.unwrap();

        drop(tx);
        worker.await.unwrap();

        let account = store.get_account(account.client).await.unwrap();
        assert_eq!(account.available, 5.0);
        assert_eq!(account.held, 0.0);
        assert_eq!(account.total, 5.0);
    }

    #[traced_test]
    #[test]
    fn test_withdrawal_insufficient_funds() {
        let rt = Arc::new(models::infra::get_runtime(1, 1, create_span()).unwrap());
        let rtc = rt.clone();
        let account = Account::load(1, 5.0, 0.0, false);
        let store = MemStore::default();
        rt.block_on(run_withdrawal_insufficient_funds_test(account, store, rtc));
        assert!(logs_contain(format!("Insufficient available funds").as_str()));
    }

    async fn run_withdrawal_insufficient_funds_test(account: Account, store: MemStore, rt: Arc<SpannedRuntime>) {
        store.update_account(&account).await.unwrap();
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let worker = Engine::new(store.clone()).start(rt.clone(), rx).await;
        tx.send(Transaction::new(TransactionKind::Withdrawal, 1, 1, Some(10.0))).await.unwrap();

        drop(tx);
        worker.await.unwrap();

        let account = store.get_account(account.client).await.unwrap();
        assert_eq!(account.available, 5.0);
        assert_eq!(account.held, 0.0);
        assert_eq!(account.total, 5.0);
    }

    #[test]
    fn test_locked_account() {
        let span = create_span();
        let rt = Arc::new(models::infra::get_runtime(1, 1, span).unwrap());
        let rtc = rt.clone();
        let account = Account::load(1, 10.0, 0.0, true);
        let store = MemStore::default();
        rt.block_on(run_locked_account_test(account, store, rtc))
    }

    async fn run_locked_account_test(account: Account, store: MemStore, rt: Arc<SpannedRuntime>) {
        store.update_account(&account).await.unwrap();
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let worker = Engine::new(store.clone()).start(rt.clone(), rx).await;

        tx.send(Transaction::new(TransactionKind::Deposit, 1, 2, Some(10.0))).await.unwrap();
        tx.send(Transaction::new(TransactionKind::Withdrawal, 1, 3, Some(10.0))).await.unwrap();
        tx.send(Transaction::new(TransactionKind::Resolve, 1, 1, None)).await.unwrap();
        tx.send(Transaction::new(TransactionKind::ChargeBack, 1, 1, None)).await.unwrap();

        drop(tx);
        worker.await.unwrap();

        let account = store.get_account(account.client).await.unwrap();
        assert_eq!(account.available, 10.0);
        assert_eq!(account.held, 0.0);
        assert_eq!(account.total, 10.0);
        assert!(account.locked);
    }

    #[test]
    fn test_dispute() {
        let span = create_span();
        let rt = Arc::new(models::infra::get_runtime(1, 1, span).unwrap());
        let rtc = rt.clone();
        let account = Account::load(1, 10.0, 0.0, false);
        let store = MemStore::default();
        rt.block_on(run_dispute_test(account, store, rtc))
    }

    async fn run_dispute_test(account: Account, store: MemStore, rt: Arc<SpannedRuntime>) {
        let txn_id = 2;
        store.add_transaction(Transaction::new(TransactionKind::Deposit, 1, txn_id, Some(10.0))).await.unwrap();
        store.update_account(&account).await.unwrap();
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let worker = Engine::new(store.clone()).start(rt.clone(), rx).await;

        tx.send(Transaction::new(TransactionKind::Dispute, 1, 2, None)).await.unwrap();

        drop(tx);
        worker.await.unwrap();

        let transaction = store.get_transaction(txn_id).await.unwrap();
        assert_eq!(transaction.under_dispute, true);

        let account = store.get_account(account.client).await.unwrap();
        assert_eq!(account.available, 0.0);
        assert_eq!(account.held, 10.0);
        assert_eq!(account.total, 10.0);
    }

    #[traced_test]
    #[test]
    fn test_dispute_on_wrong_transaction() {
        let span = create_span();
        let rt = Arc::new(models::infra::get_runtime(1, 1, span).unwrap());
        let rtc = rt.clone();
        let account = Account::load(1, 10.0, 0.0, false);
        let store = MemStore::default();
        rt.block_on(run_dispute_on_wrong_transaction_test(account, store, rtc));
        assert!(logs_contain(format!("Ignoring dispute no reference found for transaction").as_str()));
    }

    async fn run_dispute_on_wrong_transaction_test(account: Account, store: MemStore, rt: Arc<SpannedRuntime>) {
        store.add_transaction(Transaction::new(TransactionKind::Deposit, 1, 1, Some(10.0))).await.unwrap();
        store.add_transaction(Transaction::new(TransactionKind::Withdrawal, 1, 2, Some(10.0))).await.unwrap();
        store.update_account(&account).await.unwrap();
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let worker = Engine::new(store.clone()).start(rt.clone(), rx).await;

        tx.send(Transaction::new(TransactionKind::Dispute, 1, 2, None)).await.unwrap();

        drop(tx);
        worker.await.unwrap();

        let account = store.get_account(account.client).await.unwrap();
        assert_eq!(account.available, 10.0);
        assert_eq!(account.held, 0.0);
        assert_eq!(account.total, 10.0);
    }

    #[traced_test]
    #[test]
    fn test_dispute_on_transaction_already_under_dispute() {
        let span = create_span();
        let rt = Arc::new(models::infra::get_runtime(1, 1, span).unwrap());
        let rtc = rt.clone();
        let account = Account::load(1, 10.0, 0.0, false);
        let store = MemStore::default();
        rt.block_on(run_dispute_on_transaction_already_under_dispute_test(account, store, rtc));
        assert!(logs_contain(format!("Double dispute for tx").as_str()));
    }

    async fn run_dispute_on_transaction_already_under_dispute_test(account: Account, store: MemStore, rt: Arc<SpannedRuntime>) {
        let mut txn = Transaction::new(TransactionKind::Deposit, 1, 2, Some(10.0));
        txn.set_under_dispute(true);
        store.add_transaction(txn).await.unwrap();
        store.update_account(&account).await.unwrap();
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let worker = Engine::new(store.clone()).start(rt.clone(), rx).await;

        tx.send(Transaction::new(TransactionKind::Dispute, 1, 2, None)).await.unwrap();

        drop(tx);
        worker.await.unwrap();

        let account = store.get_account(account.client).await.unwrap();
        assert_eq!(account.available, 10.0);
        assert_eq!(account.held, 0.0);
        assert_eq!(account.total, 10.0);
    }

    #[traced_test]
    #[test]
    fn test_dispute_with_insufficient_balance() {
        let span = create_span();
        let rt = Arc::new(models::infra::get_runtime(1, 1, span).unwrap());
        let rtc = rt.clone();
        let account = Account::load(1, 5.0, 0.0, false);
        let store = MemStore::default();
        rt.block_on(run_dispute_with_insufficient_balance_test(account, store, rtc));
        assert!(logs_contain(format!("Insufficient available funds").as_str()));
    }

    async fn run_dispute_with_insufficient_balance_test(account: Account, store: MemStore, rt: Arc<SpannedRuntime>) {
        store.add_transaction(Transaction::new(TransactionKind::Deposit, 1, 2, Some(10.0))).await.unwrap();
        store.update_account(&account).await.unwrap();
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let worker = Engine::new(store.clone()).start(rt.clone(), rx).await;

        tx.send(Transaction::new(TransactionKind::Dispute, 1, 2, None)).await.unwrap();

        drop(tx);
        worker.await.unwrap();

        let account = store.get_account(account.client).await.unwrap();
        assert_eq!(account.available, 5.0);
        assert_eq!(account.held, 0.0);
        assert_eq!(account.total, 5.0);
    }

    #[traced_test]
    #[test]
    fn test_dispute_on_wrong_clientid() {
        let span = create_span();
        let rt = Arc::new(models::infra::get_runtime(1, 1, span).unwrap());
        let rtc = rt.clone();
        let account_1 = Account::load(1, 10.0, 0.0, false);
        let account_2 = Account::load(2, 10.0, 0.0, false);
        let store = MemStore::default();
        rt.block_on(run_dispute_on_wrong_clientid_test(account_1, account_2, store, rtc));
        assert!(logs_contain(format!("Wrong client_id in transaction").as_str()));
    }

    async fn run_dispute_on_wrong_clientid_test(account_1: Account, account_2: Account, store: MemStore, rt: Arc<SpannedRuntime>) {
        store.add_transaction(Transaction::new(TransactionKind::Deposit, 1, 1, Some(10.0))).await.unwrap();
        store.add_transaction(Transaction::new(TransactionKind::Deposit, 2, 2, Some(10.0))).await.unwrap();
        store.update_account(&account_1).await.unwrap();
        store.update_account(&account_2).await.unwrap();
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let worker = Engine::new(store.clone()).start(rt.clone(), rx).await;

        tx.send(Transaction::new(TransactionKind::Dispute, 1, 2, None)).await.unwrap();

        drop(tx);
        worker.await.unwrap();

        let transaction = store.get_transaction(1).await.unwrap();
        assert_eq!(transaction.under_dispute, false);

        let transaction = store.get_transaction(2).await.unwrap();
        assert_eq!(transaction.under_dispute, false);
    }
}