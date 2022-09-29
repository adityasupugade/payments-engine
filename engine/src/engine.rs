use core::{transactions::{Transaction, TransactionKind}, error::{Error, ErrorKind}, account::Account};
use std::sync::Arc;

use mem_store::mem_store::MemStore;
use tokio::{runtime::Runtime, sync::mpsc::Receiver};

#[derive(Clone)]

pub struct Engine {
    store: MemStore,
}

impl Engine {
    pub fn new(store: MemStore) -> Self {
        Engine{store}
    }

    pub async fn start(&self, rt: Arc<Runtime>, rx : Receiver<Transaction>) -> tokio::task::JoinHandle<()> {
        let e = self.clone();
        rt.spawn(async move { let _ = Engine::process_txn(&e, rx).await; })
    }

    async fn process_txn(&self, mut rx : Receiver<Transaction>) -> Result<(), Error> {
        while let Some(transaction) = rx.recv().await {
            print!("{:?}", transaction);
            if !transaction.is_valid_amount() {
                tracing::error!("Transaction with id {} has negative amount",transaction.id);
                continue;
            }

            if let Err(_) = self.store.add_transaction(transaction.clone()).await {
                tracing::error!("Failed to add transaction with id {}.",transaction.id);
                continue;
            }

            let transaction_result: Result<(), Error> = async {
                let mut account = self.store.get_account(transaction.client_id).await?;
                if account.locked {
                    tracing::error!("Account locked for client id {} transaction id {}", transaction.client_id, transaction.id);
                    return Err(Error::new(ErrorKind::Unknown("abc".to_string())));
                }

                self.apply_transaction(&mut account, &transaction).await?;

                self.store.update_account(&account).await?;
                Ok(())
            }.await;

            match transaction_result {
                Ok(_) => {},
                Err(_) => {
                    // let's rollback the stored transaction.
                    match transaction.kind {
                        TransactionKind::Deposit | TransactionKind::Withdrawal => {
                            // tracing::warn!("Rolling back transaction for tx {}", transaction.id);
                            if let Err(_) = self.store.delete_transaction(transaction.id).await {
                                tracing::error!("CRITICAL: Failed to rollback transaction: {}", transaction.id);
                                // return Err(Error::new(ErrorKind::Unknown("abc".to_string())));
                            }

                        },
                        TransactionKind::Dispute => {
                            if let Err(_) = self.store.set_transaction_under_dispute(transaction.id, false).await {
                                tracing::error!("CRITICAL: Failed to rollback transaction: {}", transaction.id);
                                // return Err(Error::new(ErrorKind::Unknown("abc".to_string())));
                            }
                        },
                        TransactionKind::Resolve | TransactionKind::ChargeBack => {
                            if let Err(_) = self.store.set_transaction_under_dispute(transaction.id, true).await {
                                tracing::error!("CRITICAL: Failed to rollback transaction: {}", transaction.id);
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
            return Err(Error::new(ErrorKind::Unknown("Insufficient available funds".to_string())));
        }
        account.available -= amount;
        account.total -= amount;
        Ok(())
    }

    async fn dispute(&self, account: &mut Account, info: &Transaction) -> Result<(), Error> {
        // if no ref, ignore
        let ref_transaction: Result<Transaction, Error> = self.store.get_transaction(info.id).await;
        match ref_transaction {
            Err(e) => {
                match *e.kind {
                    // ErrorKind::StoreError::NotFound(s) => {
                    //     tracing::info!("Ignoring dispute for transaction {}. No ref found", id);
                    //     Ok(())
                    // },
                    _ => return Err(e),
                }
                
            },
            Ok(ref_tx) => {
                if ref_tx.kind == TransactionKind::Deposit {
                    if account.client != info.client_id {
                        return Err(Error::new(ErrorKind::Unknown("wrong_client_error(account, &info)".to_string())));
                    } else if ref_tx.under_dispute {
                        tracing::error!(?account, "Double dispute for tx {}", info.id);
                        return Err(Error::new(ErrorKind::Unknown("DoubleDispute { id: info.id }".to_string())));
                    } else if account.available < ref_tx.amount.unwrap() {
                        tracing::error!(?account, "Insufficient available funds");
                        return Err(Error::new(ErrorKind::Unknown("InsufficientAvailableFunds".to_string())));
                    }
                    account.available -= ref_tx.amount.unwrap();
                    account.held += ref_tx.amount.unwrap();
                    self.store.set_transaction_under_dispute(info.id, true).await?;
                } else {
                    tracing::error!("Reference transaction {} is not a Deposit", info.id);
                    return Err(Error::new(ErrorKind::Unknown("WrongTransactionRef { id: info.id }".to_string())));
                }

                Ok(())
            }
        }
    }

    async fn resolve(&self, account: &mut Account, info: &Transaction) -> Result<(), Error> {
        // if no ref, ignore
        let ref_transaction: Result<Transaction, Error> = self.store.get_transaction(info.id).await;
        match ref_transaction {
            Err(e) => {
                match *e.kind {
                    // ErrorKind::StoreError::NotFound(s) => {
                    //     tracing::info!("Ignoring resolve for transaction {}. No ref found", id);
                    //     Ok(())
                    // },
                    _ => return Err(e),
                }
                
            },
            Ok(ref_tx) => {
                if ref_tx.kind == TransactionKind::Deposit {
                    if account.client != info.client_id {
                        return Err(Error::new(ErrorKind::Unknown("wrong_client_error(account, &info)".to_string())));
                    } else if account.held < ref_tx.amount.unwrap() {
                        tracing::error!(?account, "Insufficient available funds");
                        return Err(Error::new(ErrorKind::Unknown("InsufficientAvailableFunds".to_string())));
                    } else if !ref_tx.under_dispute {
                        tracing::info!("Ignoring resolve for transaction {}. Not under dispute", info.id);
                        return Ok(());
                    }
                    account.held -= ref_tx.amount.unwrap();
                    account.available += ref_tx.amount.unwrap();
                    self.store.set_transaction_under_dispute(info.id, false).await?;
                } else {
                    tracing::error!("Reference transaction {} is not a Deposit", info.id);
                    return Err(Error::new(ErrorKind::Unknown("WrongTransactionRef { id: info.id }".to_string())));
                }

                Ok(())
            }
        }
    }

    async fn chargeback(&self, account: &mut Account, info: &Transaction) -> Result<(), Error> {
        // if no ref, ignore
        let ref_transaction: Result<Transaction, Error> = self.store.get_transaction(info.id).await;
        match ref_transaction {
            Err(e) => {
                match *e.kind {
                    // ErrorKind::StoreError::NotFound(s) => {
                    //     tracing::info!("Ignoring chargeback for transaction {}. No ref found", id);
                    //     Ok(())
                    // },
                    _ => return Err(e),
                }
                
            },
            Ok(ref_tx) => {
                if ref_tx.kind == TransactionKind::Deposit {

                    if account.client != info.client_id {
                        return Err(Error::new(ErrorKind::Unknown("wrong_client_error(account, &info)".to_string())));
                    } else if account.held < ref_tx.amount.unwrap() {
                        tracing::error!(?account, "Insufficient available funds");
                        return Err(Error::new(ErrorKind::Unknown("InsufficientAvailableFunds".to_string())));
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
                    return Err(Error::new(ErrorKind::Unknown("WrongTransactionRef { id: info.id }".to_string())));
                }

                Ok(())
            }
        }
    }
}