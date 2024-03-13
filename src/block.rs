// use crate::util::Util;
use crate::wallet::Wallet;
use crate::{block, transaction::Transaction};
use log::info;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    // pub id: usize,
    // pub hash: String,
    // pub previous_hash: String,
    // pub timestamp: i64,
    // pub txn: Vec<Transaction>,
    // pub validator: String,
    // pub signature: String,
    // pub difficulty: u32,
}