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