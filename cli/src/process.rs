use models::{error::Error, infra::SpannedRuntime};
use std::sync::Arc;

use mem_store::mem_store::MemStore;
use publish::publish::Publisher;
use tokio_stream::StreamExt;
use csv::{reader::{Reader, read_csv}, writer::{write_csv, Writer}};


pub async fn process_transactions(reader: &mut Reader, store: MemStore, writer: &mut Writer, rt: Arc<SpannedRuntime>, worker_count: u16) -> Result<(), Error> {

    let mut rdr = read_csv(reader).await;
    let mut publisher = Publisher::new(store, rt, worker_count);
    while let Some(t) = rdr.next().await {
        publisher.post_txn(t.unwrap()).await?;
    }
    publisher.shutdown_gracefully().await;
    let report = publisher.get_report().await?;
    write_csv(writer, report).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use futures_util::stream::FuturesUnordered;
    use futures_util::StreamExt;

    use mem_store::mem_store::MemStore;
    use models::{logger::create_span, infra::SpannedRuntime};
    use tokio::io::BufWriter;

    use super::process_transactions;

    // This tests parallelly starts multiple process_transactions csv.
    #[test]
    fn test_process_transactions() {
        let rt = Arc::new(models::infra::get_runtime(1, 1, create_span()).unwrap());
        let rtc = rt.clone();

        let mut output1 = BufWriter::new(Vec::<u8>::new());
        let mut output2 = BufWriter::new(Vec::<u8>::new());
        let mut output3 = BufWriter::new(Vec::<u8>::new());
        
        rt.block_on(run_process_transactions_test(&mut output1, &mut output2, &mut output3, rtc));

        let buffer = output1.into_inner();
        let csv = String::from_utf8_lossy(&buffer);
        
        let expected = (csv
            == "client,available,held,total,locked\n1,250.0,0.0,250.0,false\n2,0.0,0.0,0.0,true\n")
            || (csv == "client,available,held,total,locked\n2,0.0,0.0,0.0,true\n1,250.0,0.0,250.0,false\n");

        assert!(expected);

        let buffer = output2.into_inner();
        let csv = String::from_utf8_lossy(&buffer);
        
        let expected = (csv
            == "client,available,held,total,locked\n1,250.0,0.0,250.0,false\n2,0.0,0.0,0.0,true\n")
            || (csv == "client,available,held,total,locked\n2,0.0,0.0,0.0,true\n1,250.0,0.0,250.0,false\n");

        assert!(expected);

        let buffer = output3.into_inner();
        let csv = String::from_utf8_lossy(&buffer);
        
        let expected = (csv
            == "client,available,held,total,locked\n1,250.0,0.0,250.0,false\n2,0.0,0.0,0.0,true\n")
            || (csv == "client,available,held,total,locked\n2,0.0,0.0,0.0,true\n1,250.0,0.0,250.0,false\n");

        assert!(expected);
    }

    async fn run_process_transactions_test(output1: &mut BufWriter<Vec<u8>>, output2: &mut BufWriter<Vec<u8>>, output3: &mut BufWriter<Vec<u8>>, rt: Arc<SpannedRuntime>) {

        let mut input1 = r"
        type,client,tx,amount
        deposit,1,1,100
        withdrawal,1,2,50
        deposit,2,3,100
        deposit,1,4,200
        dispute,1,4
        resolve,1,4
        dispute,2,3
        chargeback,2,3
        dispute,1,3"
            .as_bytes();

        let mut input2 = r"
        type,client,tx,amount
        deposit,1,1,100
        withdrawal,1,2,50
        deposit,2,3,100
        deposit,1,4,200
        dispute,1,4
        resolve,1,4
        dispute,2,3
        chargeback,2,3
        dispute,1,3"
            .as_bytes();

        let mut input3 = r"
        type,client,tx,amount
        deposit,1,1,100
        withdrawal,1,2,50
        deposit,2,3,100
        deposit,1,4,200
        dispute,1,4
        resolve,1,4
        dispute,2,3
        chargeback,2,3
        dispute,1,3"
            .as_bytes();

        let mut futures = FuturesUnordered::new();
        let rtc = rt.clone();
        let store1 = MemStore::default();
        let store2 = MemStore::default();
        let store3 = MemStore::default();

        let fut1 = process_transactions(&mut input1, store1, output1, rtc.clone(), 2);
        let fut2 = process_transactions(&mut input2, store2, output2, rtc.clone(), 2);
        let fut3 = process_transactions(&mut input3, store3, output3, rtc, 2);
        futures.push(fut1);
        futures.push(fut2);
        futures.push(fut3);

        while let Some(_) = futures.next().await {}
    }
}