use core::{transactions::{Transaction, TransactionKind}, account::Account, error::{Error, ErrorKind}};
use std::{collections::HashMap, sync::Arc};

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

impl MemStore {
    pub async fn add_transaction(&self, transaction: Transaction) -> Result<Transaction, Error> {
        // tracing::debug!("Creating transaction: {:?}", transaction);
        if let TransactionKind::Deposit = transaction.kind {
            let mut result = self
                .transactions
                .write().await;

            match result.entry(transaction.id) {
                std::collections::hash_map::Entry::Occupied(_) => Err(Error::new(ErrorKind::Unknown("a".to_string()))),
                std::collections::hash_map::Entry::Vacant(_) => Ok(result.insert(transaction.id, transaction).unwrap()),
            }
        } else {
            Ok(transaction)
        }
    }

    pub async fn get_transaction(&self, id: u32) -> Result<Transaction, Error> {
        // tracing::debug!("Getting transaction {}", id);
        let result = self
            .transactions
            .read().await;

        let a = match result.get(&id) {
            Some(t) => Ok(t.clone()),
            None => Err(Error::new(ErrorKind::Unknown("a".to_string()))),
        };
        a
    }

    pub async fn get_account(&self, id: u16) -> Result<Account, Error> {
        // tracing::debug!("Getting account: {}", id);
        let result = self
            .accounts
            .read().await;

        let a = result.get(&id);
        match a {
            Some(a) => return Ok(a.clone()),
            None => return Ok(Account::new(id)),
        }
    }

}