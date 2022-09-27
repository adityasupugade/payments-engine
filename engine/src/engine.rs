use core::{transactions::Transaction, error::Error};
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
        rt.spawn(async move { let _ = Engine::process_txn(rx).await; })
    }

    async fn process_txn(mut rx : Receiver<Transaction>) -> Result<(), Error> {
        loop {
            let txn = rx.recv().await;
            if txn.is_none() {
                break;
            }
            // process txn
        }
        Ok(())
    }
}