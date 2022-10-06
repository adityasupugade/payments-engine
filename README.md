# payments-engine
This is simple payment engine which processes following types of transaction:
- **Deposit**: A deposit is a credit to the client's account.
- **Withdraw**: A withdraw is a debit to the client's asset account,
- **Dispute**: A dispute represents a client's claim that a transaction was erroneous and should be reversed.
- **Resolve**: A resolve represents a resolution to a dispute.
- **Chargeback**: A chargeback is the final state of a dispute and represents the client reversing a transaction.

## Execute
To run this project, you can use the following command:
> cargo run -- transactions.csv > accounts.csv

## Input
The input will be a CSV file with the columns type, client, tx, and amount.
For example
>type, client, tx, amount
>deposit, 1, 1, 1.0
>deposit, 2, 2, 2.0
>deposit, 1, 3, 2.0
>withdrawal, 1, 4, 1.5
>withdrawal, 2, 5, 3.0

## Output
The output should be a list of client IDs (client), available amounts (available), held amounts
(held), total amounts (total), and whether the account is locked (locked).

For example
>client, available, held, total, locked
>1, 1.5, 0.0, 1.5, false
>2, 2.0, 0.0, 2.0, false

## Assumptions
* Transactions IDs are global and unique.
* Withdrawals cannot be disputed, only deposits.
* Transactions to a locked account are ignored.

## Architecture
This project has several crates to make project modular, more maintainable and extensible.
- **cli**: Cli is entry point to the app through CLI. 
- **csv**: Csv crate provides reader and writer interfaces to csv file. csv_async crate is used to perform this operations in async way.
- **publisher**: Publisher is responsible to provide transactions to engine for processing and manage parallelism via multi worker.
- **engine**: Engine processes each transaction and updates its result to mem store.
- **mem store**: Mem store maintains account information for each client and transaction info as well.
- **models**: Models provides all common functionality, structures used accross all crates.
![Flow Diagram](/Payment_Engine_Architecture.jpg)

## Parallelism
Single CSV file can be processed by multiple workers. This parallelism achieved via channels.
Transactions for single client are processed sequentially to avoid any race conditions. But transactions having different client id can be processed parallelly.
In Publisher crate, mapping of client id and its corresponding engine channel is stored.
client Id is currently sharded using % worker count, this can further be improved to balance load evenly across engine workers.

## Testing
Added unit testcases in each trait.
End to end testing is done manually, providing csv input files used for the same.

## Observability
All the libraries used in this project are using tracing to provide observability.

## Error handling
Currently all errors are logged in tracing and errors are ignored in CLI. 
If there is any processing needs to be done in future, errors can be propogated to main thread via channels.

## Scaling
This code can be used on server which accepts concurrent TCP streams. 
Based on server configuration, number of process workers can be defined, also number of engine workers can be configured.
Process workers will pick tcp stream for processing and return the response.
Similar test is written in cli crate which parallelly process multiple requests.