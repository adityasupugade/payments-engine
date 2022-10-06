# payments-engine
This is simple payment engine which processes following types of transaction:
- **Deposit**: A deposit is a credit to the client's account.
- **Withdraw**: A withdraw is a debit to the client's asset account,
- **Dispute**: A dispute represents a client's claim that a transaction was erroneous and should be reversed.
- **Resolve**: A resolve represents a resolution to a dispute.
- **Chargeback**: A chargeback is the final state of a dispute and represents the client reversing a transaction.

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

## Assumptions:
* Transactions IDs are global and unique.
* Withdrawals cannot be disputed, only deposits.
* Transactions to a locked account are ignored.

## Architecture:
![Flow Diagram](https://github.com/adityasupugade/payments-engine/Payment_Engine_Architecture.jpg)

## Testing:

## Scaling:
