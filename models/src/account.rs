use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Account {
    pub client: u16,
    pub available: f32,
    pub held: f32,
    pub total: f32,
    pub locked: bool,
}

impl Account {
    pub const fn new(client: u16) -> Self {
        Self {
            client,
            available: 0.0,
            held: 0.0,
            total: 0.0,
            locked: false,
        }
    }

    pub fn load(client: u16, available: f32, held: f32, locked: bool) -> Self {
        Self {
            client,
            available,
            held,
            total: available + held,
            locked,
        }
    }

    pub fn to_max_display_precision(&mut self) {
        self.available = truncate(&self.available);
        self.held = truncate(&self.held);
        self.total = truncate(&self.total);
    }
}

fn truncate(num: &f32) -> f32 {
    let s = format!("{:.4}", num);
    s.parse::<f32>().expect("truncate failed")
}