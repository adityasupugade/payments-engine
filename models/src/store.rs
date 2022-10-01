use async_trait::async_trait;

use crate::account::Account;
use crate::transactions::Transaction;
use crate::error::Error;

#[async_trait]
pub trait Store: Send + Sync {
    async fn add_transaction(&self, transaction: Transaction) -> Result<Transaction, Error>;
    async fn get_transaction(&self, id: u32) -> Result<Transaction, Error>;
    async fn delete_transaction(&self, id: u32) -> Result<(), Error>;
    async fn set_transaction_under_dispute(&self, id: u32, under_dispute: bool) -> Result<(), Error>;
    async fn get_account(&self, id: u16) -> Result<Account, Error>;
    async fn update_account(&self, account: &Account) -> Result<(), Error>;
}
