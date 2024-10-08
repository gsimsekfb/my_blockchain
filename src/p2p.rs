// use super::{App, Block};
use crate::{
    block::Block, blockchain::Blockchain, transaction::{self, Transaction}, util::peer_id_short, wallet::Wallet
};

use libp2p::{
    floodsub::{Floodsub, FloodsubEvent, Topic},
    identity,
    mdns::{Mdns, MdnsEvent},
    swarm::{NetworkBehaviourEventProcess, Swarm},
    NetworkBehaviour, PeerId,
};
use log::{info, warn};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::sync::mpsc;

// // let base_64_encoded = "CAESQL6vdKQuznQosTrW7FWI9At+XX7EBf0BnZLhb6w+N+XSQSdfInl6c7U4NuxXJlhKcRBlBw9d0tj2dfBIVf6mcPA=";
// // let encoded = base64::decode(base_64_encoded).unwrap();
// // let keypair = Keypair::from_protobuf_encoding(&encoded).unwrap();
// pub static KEYS: Lazy<identity::Keypair> = Lazy::new(
//     || identity::Keypair::from_protobuf_encoding(
//         &base64::decode("CAESQL6vdKQuznQosTrW7FWI9At+XX7EBf0BnZLhb6w+N+XSQSdfInl6c7U4NuxXJlhKcRBlBw9d0tj2dfBIVf6mcPA=").unwrap()
//     ).unwrap()
// );

pub static KEYS: Lazy<identity::Keypair> = Lazy::new(identity::Keypair::generate_ed25519);
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));
pub static TXN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("transactions"));

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainResponse {
    pub blocks: Vec<Block>,
    pub mempool_txns: Vec<Transaction>,
    pub receiver: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainRequest {
    pub from_peer_id: String,
}

pub enum EventType {
    Input(String),
    Init,
    Mining,
}

#[derive(NetworkBehaviour)]
pub struct AppBehaviour {
    pub floodsub: Floodsub,
    pub mdns: Mdns,
    #[behaviour(ignore)]
    pub response_sender: mpsc::UnboundedSender<ChainResponse>,
    #[behaviour(ignore)]
    pub init_sender: mpsc::UnboundedSender<bool>,
    #[behaviour(ignore)]
    pub blockchain: Blockchain,
}

impl AppBehaviour {
    pub async fn new(
        blockchain: Blockchain,
        response_sender: mpsc::UnboundedSender<ChainResponse>,
        init_sender: mpsc::UnboundedSender<bool>,
    ) -> Self {
        let mut behaviour = Self {
            blockchain,
            floodsub: Floodsub::new(*PEER_ID),
            mdns: Mdns::new(Default::default())
                .await
                .expect("can create mdns"),
            response_sender,
            init_sender,
        };
        behaviour.floodsub.subscribe(CHAIN_TOPIC.clone());
        behaviour.floodsub.subscribe(BLOCK_TOPIC.clone());
        behaviour.floodsub.subscribe(TXN_TOPIC.clone());

        behaviour
    }
}

// incoming event handler
impl NetworkBehaviourEventProcess<FloodsubEvent> for AppBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        if let FloodsubEvent::Message(msg) = event {
            if let Ok(resp) = serde_json::from_slice::<ChainResponse>(&msg.data) {
                if resp.receiver == PEER_ID.to_string() {
                    info!("ChainResponse from peer *{}:", &msg.source.to_string()[52-4..]);
                    // resp.blocks.iter().for_each(|r| info!("{:?}", r));cc

                    self.blockchain.replace_chain(&resp.blocks);
                    // self.blockchain.mempool.transactions = resp
                    //     .txns
                    //     .into_iter()
                    //     .filter(|txn| Transaction::verify_txn(txn).is_ok())
                    //     .collect();
                }
            } else if let Ok(req) = serde_json::from_slice::<ChainRequest>(&msg.data) {
                info!("sending local chain & mempool to {}", 
                    peer_id_short(&msg.source));
                // Verify this request is for me
                if PEER_ID.to_string() == req.from_peer_id {
                    let json = serde_json::to_string(&ChainResponse {
                        blocks: self.blockchain.chain.clone(),
                        mempool_txns: self.blockchain.mempool.transactions.clone(),
                        receiver: msg.source.to_string(),
                    })
                    .expect("can jsonify response");

                    self.floodsub.publish(CHAIN_TOPIC.clone(), json.as_bytes());
                }
            } else if let Ok(block) = serde_json::from_slice::<Block>(&msg.data) {
                // info!("received new block from {}", msg.source.to_string());
                info!("received new block {:?}", block);
                if self.blockchain.chain.last().unwrap().id < block.id
                    && self.blockchain.is_valid_block(block.clone())
                {
                    info!("relaying new valid block");
                    let json = serde_json::to_string(&block).expect("can jsonify request");
                    self.floodsub.publish(BLOCK_TOPIC.clone(), json.as_bytes());
                }
            } else if let Ok(txn) = serde_json::from_slice::<Transaction>(&msg.data) {
                info!("received new transaction from {}", msg.source.to_string());

                if !self.blockchain.txn_exist(&txn) && Transaction::verify_txn(&txn).is_ok() {
                    info!("relaying new valid transaction");
                    let json = serde_json::to_string(&txn).expect("can jsonify request");
                    self.floodsub.publish(TXN_TOPIC.clone(), json.as_bytes());
                    self.blockchain.add_txn(txn);
                }
            }
        }
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for AppBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(discovered_list) => {
                for (peer, _addr) in discovered_list {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(expired_list) => {
                for (peer, _addr) in expired_list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}

pub fn get_list_peers(swarm: &Swarm<AppBehaviour>) -> Vec<String> {
    let nodes = swarm.behaviour().mdns.discovered_nodes();
    let mut unique_peers = HashSet::new();
    for peer in nodes {
        unique_peers.insert(peer);
    }
    let unique_peers = unique_peers.iter().map(|p| p.to_string()).collect();
    info!("discovered peers: {:?}", &unique_peers);
    unique_peers
}

pub fn handle_print_peers(swarm: &Swarm<AppBehaviour>) {
    let peers = get_list_peers(swarm);
    peers.iter().for_each(|p| info!("{}", p));
}

pub fn handle_print_chain(swarm: &Swarm<AppBehaviour>) {
    info!("Local Blockchain:");
    let pretty_json = serde_json::to_string_pretty(&swarm.behaviour().blockchain.chain)
        .expect("can jsonify blocks");
    info!("{}", pretty_json);
}

pub fn handle_print_balance(swarm: &Swarm<AppBehaviour>) {
    info!("Account Balance:");
    let pretty_json = serde_json::to_string_pretty(
        &swarm.behaviour().blockchain.accounts.balances
    ).expect("can jsonify blocks");
    info!("{}", pretty_json);
}

// pub fn handle_print_validator(swarm: &Swarm<AppBehaviour>) {
//     info!("Validators: ");
//     let pretty_json =
//         serde_json::to_string_pretty(&swarm.behaviour().blockchain.validators.accounts)
//             .expect("can jsonify blocks");
//     info!("{}", pretty_json);
// }

// pub fn handle_print_stake(swarm: &Swarm<AppBehaviour>) {
//     info!("Validators Stake: ");
//     let pretty_json = serde_json::to_string_pretty(&swarm.behaviour().blockchain.stakes.balances)
//         .expect("can jsonify blocks");
//     info!("{}", pretty_json);
// }

pub fn handle_set_wallet(cmd: &str, swarm: &mut Swarm<AppBehaviour>) {
    if let Some(data) = cmd.strip_prefix("set wallet") {
        let arg: Vec<&str> = data.split_whitespace().collect();
        let key_pair = arg.get(0).expect("No keypair found").to_string();
        info!("setting node wallet to {}", key_pair);
        swarm.behaviour_mut().blockchain.wallet = Wallet::get_wallet(key_pair);
    }
}

pub fn handle_print_wallet(swarm: &mut Swarm<AppBehaviour>) {
    let pub_key = swarm.behaviour_mut().blockchain.wallet.get_public_key();
    info!("Node wallet public key: {}", pub_key);
}

pub fn handle_print_mempool(swarm: &Swarm<AppBehaviour>) {
    let pretty_json = serde_json::to_string_pretty(
        &swarm.behaviour().blockchain.mempool.transactions
    ).expect("can jsonify blocks");
    info!("mempool: {}", pretty_json);
}

pub fn dummy_txn(cmd: &str, swarm: &mut Swarm<AppBehaviour>) {
    info!("dummy_txn new txn 2306.. 10 5aed..");
    handle_create_txn(
        "new txn \
            230681c76f00b412ccf7757a8449c448a04acd735e497a7612b66d8bfcb8e576 \
            10 \
            5aede624154386ca358af195e13a46981b917ee8279f30a67d7a211a3d3e7243", 
        swarm
    );
}

pub fn handle_create_txn(cmd: &str, swarm: &mut Swarm<AppBehaviour>) {
    if let Some(data) = cmd.strip_prefix("new txn") {
        let arg: Vec<&str> = data.split_whitespace().collect();

        let to = arg.get(0).expect("No receipient found").to_string();
        let amount = arg
            .get(1)
            .expect("No amount found")
            .to_string()
            .parse::<f64>()
            .expect("Convert amount string to float");
        let category = arg.get(2).expect("No txntype found").to_string();

        let txn_type = match category.as_str() {
            "txn" => crate::transaction::TransactionType::TRANSACTION,
            "stake" => crate::transaction::TransactionType::STAKE,
            "validator" => crate::transaction::TransactionType::VALIDATOR,
            _ => crate::transaction::TransactionType::TRANSACTION,
        };

        let behaviour = swarm.behaviour_mut();

        let mut wallet = behaviour.blockchain.wallet.clone();

        if amount + transaction::TRANSACTION_FEE
            > *behaviour.blockchain.get_balance(&wallet.get_public_key())
        {
            let bal = *behaviour.blockchain.get_balance(&wallet.get_public_key());
            let addr = wallet.get_public_key();
            warn!("Wallet has insufficient amount. 
                   amount: {amount}, bal: {bal}, addr: {addr}");
            return;
        }

        match Blockchain::create_txn(&mut wallet, to, amount, txn_type) {
            Ok(txn) => {
                let json = serde_json::to_string(&txn).expect("can jsonify request");

                info!("Adding new transaction to mempool");
                // behaviour.blockchain.mempool.add_transaction(txn);
                info!("Broadcasting new transaction");
                behaviour
                    .floodsub
                    .publish(TXN_TOPIC.clone(), json.as_bytes());
            }
            Err(_) => {
                warn!("Failed to create transaction: Unable to serialized transactions into json");
            }
        };
    }
}
