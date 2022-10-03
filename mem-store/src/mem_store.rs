use models::{transactions::{Transaction, TransactionKind}, account::Account, error::{Error, ErrorKind}, store::Store};
use std::{collections::HashMap, sync::Arc, pin::Pin};

use async_trait::async_trait;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct MemStore {
    transactions: Arc<RwLock<HashMap<u32, Transaction>>>,
    accounts: Arc<RwLock<HashMap<u16, Account>>>,
}

impl Default for MemStore {
    fn default() -> Self {
        Self {
            transactions: Arc::new(RwLock::new(HashMap::new())),
            accounts: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}


#[async_trait]
impl Store for MemStore {
    async fn add_transaction(&self, transaction: Transaction) -> Result<Transaction, Error> {
        tracing::debug!("Creating transaction: {:?}", transaction);
        if let TransactionKind::Deposit = transaction.kind {
            let mut result = self
                .transactions
                .write().await;

            match result.entry(transaction.id) {
                std::collections::hash_map::Entry::Occupied(_) => {
                    Err(Error::new(ErrorKind::StoreError("Transaction with transaction id exists.".to_string())))
                },
                std::collections::hash_map::Entry::Vacant(_) => {
                    result.insert(transaction.id, transaction.clone());
                    Ok(transaction)
                }
            }
        } else {
            Ok(transaction)
        }
    }

    async fn get_transaction(&self, id: u32) -> Result<Transaction, Error> {
        tracing::debug!("Getting transaction {}", id);
        let result = self
            .transactions
            .read().await;

        let a = match result.get(&id) {
            Some(t) => Ok(t.clone()),
            None => Err(Error::new(ErrorKind::StoreError("Transaction with transaction Id does not exist.".to_string()))),
        };
        a
    }

    async fn delete_transaction(&self, id: u32) -> Result<(), Error> {
        tracing::debug!("Deleting transaction: {:?}", id);
        let mut result = self.transactions
            .write().await;

        result.remove(&id);
        Ok(())
    }

    async fn set_transaction_under_dispute(&self, id: u32, under_dispute: bool) -> Result<(), Error> {
        tracing::debug!("Setting transaction with id {} under dispute to {}", id, under_dispute);
        let mut result = self.transactions
            .write().await;

        if let Some(transaction) = result.get_mut(&id) {
            transaction.set_under_dispute(under_dispute);
        }
        Ok(())
    }

    async fn get_account(&self, id: u16) -> Result<Account, Error> {
        tracing::debug!("Getting account: {}", id);
        let result = self
            .accounts
            .read().await;

        let a = result.get(&id);
        match a {
            Some(a) => {
                return Ok(a.clone())
            },
            None => return Ok(Account::new(id)),
        }
    }

    async fn update_account(&self, account: &Account) -> Result<(), Error> {
        tracing::debug!("Updating account: {:?}", account);
        let mut result = self
            .accounts
            .write().await;

        result.insert(account.client, account.clone());
        Ok(())
    }

    async fn get_all_accounts(&self) -> Result<Pin<Box<dyn futures::Stream<Item = Account> + Send>>, Error> {
        tracing::debug!("getting all accounts");
        let result = self
            .accounts
            .read().await;

        Ok(Box::pin(futures::stream::iter(result.values().cloned().collect::<Vec<_>>())))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use models::{transactions::{TransactionKind, Transaction}, logger::create_span, store::Store, error::{ErrorKind, Error}, account::Account};

    use super::MemStore;


    #[test]
    fn test_add_duplicate_transaction() {
        let rt = Arc::new(models::infra::get_runtime(1, 1, create_span()).unwrap());
        let store = MemStore::default();
        let txn = Transaction::new(TransactionKind::Deposit, 1, 2, Some(10.0));
        rt.block_on(run_add_duplicate_transaction_test(txn, store))
    }

    async fn run_add_duplicate_transaction_test(txn: Transaction, store: MemStore) {
        let result = store.add_transaction(txn.clone()).await;
        assert!(result.is_ok());
        let result = store.add_transaction(txn.clone()).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        let exp_err = Error::new(ErrorKind::StoreError("Transaction with transaction id exists.".to_string()));
        assert_eq!(err.to_string(), exp_err.to_string());
    }

    #[test]
    fn test_add_all_kinds_transaction() {
        let rt = Arc::new(models::infra::get_runtime(1, 1, create_span()).unwrap());
        let store = MemStore::default();
        rt.block_on(run_add_all_kinds_transaction_test(store))
    }

    async fn run_add_all_kinds_transaction_test(store: MemStore) {
        let txn1 = Transaction::new(TransactionKind::Deposit, 1, 1, Some(10.0));
        let txn2 = Transaction::new(TransactionKind::Withdrawal, 1, 2, Some(10.0));
        let txn3 = Transaction::new(TransactionKind::Dispute, 1, 3, None);
        let txn4 = Transaction::new(TransactionKind::Resolve, 1, 4, None);
        let txn5 = Transaction::new(TransactionKind::ChargeBack, 1, 5, None);

        store.add_transaction(txn1.clone()).await.unwrap();
        store.add_transaction(txn2.clone()).await.unwrap();
        store.add_transaction(txn3.clone()).await.unwrap();
        store.add_transaction(txn4.clone()).await.unwrap();
        store.add_transaction(txn5.clone()).await.unwrap();

        let t1 = store.get_transaction(txn1.id).await;
        assert!(t1.is_ok());

        let exp_err = Error::new(ErrorKind::StoreError("Transaction with transaction Id does not exist.".to_string()));

        let t2 = store.get_transaction(txn2.id).await;
        assert!(t2.is_err());
        let err = t2.unwrap_err();
        assert_eq!(err.to_string(), exp_err.to_string());

        let t3 = store.get_transaction(txn3.id).await;
        assert!(t3.is_err());
        let err = t3.unwrap_err();
        assert_eq!(err.to_string(), exp_err.to_string());

        let t4 = store.get_transaction(txn4.id).await;
        assert!(t4.is_err());
        let err = t4.unwrap_err();
        assert_eq!(err.to_string(), exp_err.to_string());

        let t5 = store.get_transaction(txn5.id).await;
        assert!(t5.is_err());
        let err = t5.unwrap_err();
        assert_eq!(err.to_string(), exp_err.to_string());
    }

    #[test]
    fn test_delete_transaction() {
        let rt = Arc::new(models::infra::get_runtime(1, 1, create_span()).unwrap());
        let store = MemStore::default();
        let txn = Transaction::new(TransactionKind::Deposit, 1, 2, Some(10.0));
        rt.block_on(run_delete_transaction_test(txn, store))
    }

    async fn run_delete_transaction_test(txn: Transaction, store: MemStore) {
        let result = store.add_transaction(txn.clone()).await;
        assert!(result.is_ok());
        let result = store.delete_transaction(txn.id).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_account() {
        let rt = Arc::new(models::infra::get_runtime(1, 1, create_span()).unwrap());
        let store = MemStore::default();
        let account = Account::load(1, 10.0, 0.0, false);
        rt.block_on(run_account_test(account, store))
    }

    async fn run_account_test(account: Account, store: MemStore) {
        let result = store.update_account(&account).await;
        assert!(result.is_ok());
        let result = store.get_account(account.client).await;
        assert!(result.is_ok());
    }
}