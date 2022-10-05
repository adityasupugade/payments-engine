use models::{transactions::Transaction, error::{Error, ErrorKind}, account::Account, infra::SpannedRuntime};
use std::{collections::HashMap, sync::Arc, pin::Pin};

use engine::engine::Engine;
use mem_store::mem_store::MemStore;
use tokio::{sync::{mpsc::Sender, Mutex}, task::JoinHandle};

pub struct Publisher {
    client_sender_map: HashMap<u16, Sender<Transaction>>,
    mem_store: MemStore,
    rt: Arc<SpannedRuntime>,
    worker_count: u16,
    pub workers: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl Publisher {
    pub fn new(mem_store: MemStore, rt: Arc<SpannedRuntime>, worker_count: u16) -> Self {
        Self{client_sender_map: HashMap::new(), mem_store, rt, worker_count, workers: Arc::new(Mutex::new(Vec::new()))}
    }

    // Post transaction will send the given transaction on engine processing
    // channel based on client ID sharded with worked count.
    // Transactions for different clients will be processed parallelly,
    // and transaction for single client will be processed sequentially.
    pub async fn post_txn(&mut self, transaction: Transaction) -> Result<(), Error> {
        
        match self.client_sender_map.get(&(&transaction.client_id % self.worker_count)) {
            Some(tx) => {
                tx.send(transaction).await?
            },
            None => {
                // Spawn new worker.
                tracing::info!("Spawning new payment engine worker");
                let (tx, rx) = tokio::sync::mpsc::channel(10);
                let worker = Engine::new(self.mem_store.clone()).start(self.rt.clone(), rx).await;
                tx.send(transaction.clone()).await?;
                self.workers.lock().await.push(worker);
                self.client_sender_map.insert(transaction.client_id % self.worker_count, tx);
                
            },
        }
        Ok(())
    }

    // shutdown_gracefully will wait until all workers finish processing.
    pub async fn shutdown_gracefully(&mut self) -> Vec<Result<(), Error>> {
        self.client_sender_map.clear();
        let mut results = Vec::new();
        for worker in self.workers.lock().await
            .iter_mut()
        {
            results.push(worker.await.map_err(|e|
                Error::new(ErrorKind::JoinError(e))
            ));
        }
        tracing::info!("Stopped all payment engine workers");
        return results;
    }

    pub async fn get_report(&mut self) -> Result<Pin<Box<dyn futures::Stream<Item = Account> + Send>>, Error> {
        let engine = Engine::new(self.mem_store.clone());
        engine.report().await
    }
}