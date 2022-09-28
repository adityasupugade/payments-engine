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

    pub async fn start(&self, rt: Arc<Runtime>, mut rx : Receiver<Transaction>) -> tokio::task::JoinHandle<()> {
        let e = self.clone();
        rt.spawn(async move { let _ = Engine::process_txn(&e, rx).await; })
    }

    async fn process_txn(&self, mut rx : Receiver<Transaction>) -> Result<(), Error> {
        while let Some(transaction) = rx.recv().await {
            print!("{:?}", transaction);
            if !transaction.is_valid_amount() {
                tracing::error!(
                    "Transaction with id {} has negative amount",
                    transaction.id
                );
                continue;
            }

            self.store.add_transaction(transaction.clone()).await?;

            let transaction_result: Result<(), Error> = async {
                let mut account = self.store.get_account(transaction.client_id).await?;
                if account.locked {
                    tracing::error!("Account locked for client id {} transaction id {}",
                        transaction.client_id, transaction.id);
                    return Err(Error::new(ErrorKind::Unknown("abc".to_string())));
                }

                self.apply_transaction(&mut account, &transaction).await?;

                Ok(())
            }.await;
        }
        Ok(())
    }

    pub async fn apply_transaction(&self, account: &mut Account, transaction: &Transaction) -> Result<(), Error> {
        match transaction.kind {
            TransactionKind::Deposit => { return self.deposit(account, &transaction.amount.unwrap()).await;},
            TransactionKind::Withdrawal => { return self.withdrawal(account, &transaction.amount.unwrap()).await;},
            TransactionKind::Dispute => { return self.dispute(account, &transaction).await;},
            TransactionKind::Resolve => todo!(),
            TransactionKind::ChargeBack => todo!(),
        }
        Ok(())
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
                    // ErrorKind::Unknown(s) => {
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
                    // if everything is fine: update the account
                    account.available -= ref_tx.amount.unwrap();
                    account.held += ref_tx.amount.unwrap();
                    // set to under dispute
                    // self.store
                    //     .set_transaction_under_dispute(info.id, true)
                    //     .await?;
                } else {
                    tracing::error!("Reference transaction {} is not a Deposit", info.id);
                    return Err(Error::new(ErrorKind::Unknown("WrongTransactionRef { id: info.id }".to_string())));
                }

                Ok(())
            }
        }
    }
}