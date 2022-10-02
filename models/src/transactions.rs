use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionKind {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    ChargeBack,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub kind: TransactionKind,
    #[serde(rename = "client")]
    pub client_id: u16,
    #[serde(rename = "tx")]
    pub id: u32,
    #[serde(default)]
    pub amount: Option<f32>,
    #[serde(skip)]
    pub under_dispute: bool,
}

impl Transaction {
    pub const fn new(kind: TransactionKind, client_id: u16, id: u32, amount: Option<f32>) -> Self {
        Self {  kind,
                client_id,
                id,
                amount,
                under_dispute: false
             }
    }

    pub fn is_valid_amount(&self) -> bool {
        match self.amount {
            Some(a) => {
                if a < 0.0 {
                    return false;
                } else {
                    return true;
                }
            },
            None => return true,
        }
    }

    pub fn set_under_dispute(&mut self, under_dispute: bool) {
        self.under_dispute = under_dispute;
    }
}