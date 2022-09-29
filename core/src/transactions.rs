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
    #[serde(skip_serializing)]
    pub under_dispute: bool,
}

impl Transaction {
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