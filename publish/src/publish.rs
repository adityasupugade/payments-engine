use models::{transactions::Transaction, error::{Error, ErrorKind}};
use std::{collections::HashMap, sync::{Arc, Mutex}};

use engine::engine::Engine;
use mem_store::mem_store::MemStore;
use tokio::{sync::mpsc::Sender, runtime::Runtime, task::JoinHandle};

pub struct Publisher {
    map: HashMap<u16, Sender<Transaction>>,
    mem_store: MemStore,
    rt: Arc<Runtime>,
    worker_count: u16,
    pub workers: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl Publisher {
    pub fn new(mem_store: MemStore, rt: Arc<Runtime>, worker_count: u16) -> Self {
        Self{map: HashMap::new(), mem_store, rt, worker_count, workers: Arc::new(Mutex::new(Vec::new()))}
    }

    pub async fn post_txn(&mut self, transaction: Transaction) -> Result<(), Error> {
        
        match self.map.get(&(&transaction.client_id % self.worker_count)) {
            Some(tx) => {
                tx.send(transaction).await?
            },
            None => {
                let (tx, rx) = tokio::sync::mpsc::channel(10);
                let worker = Engine::new(self.mem_store.clone()).start(self.rt.clone(), rx).await;
                tx.send(transaction.clone()).await?;
                self.workers.lock().expect("").push(worker);
                self.map.insert(transaction.client_id % self.worker_count, tx);
                
            },
        }
        Ok(())
    }

    pub async fn shutdown_gracefully(&mut self) -> Vec<Result<(), Error>> {
        self.map.clear();
        let mut results = Vec::new();
        for worker in self
            .workers
            .lock()
            .expect("Ignore lock poisoning")
            .iter_mut()
        {
            match worker.await {
                Ok(result) => {
                    println!("{:?}", result);
                    results.push(Ok(()))
                },
                Err(join_error) => {
                    println!("{:?}", join_error);
                    // Err(Error::new(ErrorKind::Unknown("error".to_string())));
                },
            }
        }
        return results;
    }
}