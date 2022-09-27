use core::{transactions::Transaction, account::Account};
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
    
}