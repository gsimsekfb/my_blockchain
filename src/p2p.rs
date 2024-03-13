use crate::{
    block::Block, blockchain::Blockchain, transaction, transaction::Transaction,
    wallet::Wallet,
};
use std::collections::HashSet;
use libp2p::{
    floodsub::{Floodsub, FloodsubEvent, Topic},
    identity::Keypair,
    mdns::{Mdns, MdnsEvent},
    swarm::{NetworkBehaviourEventProcess, Swarm},
    NetworkBehaviour, PeerId,
};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use once_cell::sync::Lazy;
use tokio::sync::mpsc;

pub static KEYS: Lazy<Keypair> = Lazy::new(Keypair::generate_ed25519);
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));

pub fn get_list_peers(swarm: &Swarm<AppNetworkBehavior>) -> Vec<String> {
    info!("Discovered Peers:");
    let nodes = swarm.behaviour().mdns.discovered_nodes();
    let mut unique_peers = HashSet::new();
    for peer in nodes {
        unique_peers.insert(peer);
    }
    unique_peers.iter().map(|p| p.to_string()).collect()
}

// #[derive(Debug, Serialize, Deserialize)]
pub struct ChainResponse {
    pub blocks: Vec<Block>,
    pub txns: Vec<Transaction>,
    pub receiver: String,
}

#[derive(NetworkBehaviour)]
pub struct AppNetworkBehavior { // todo: rename AppNetworkBehaviour ?
    // pub floodsub: Floodsub, // handles the Floodsub protocol
    //                         // which is a message broadcast protocol
    pub mdns: Mdns, // Automatically discovers peers on the local network
    // #[behaviour(ignore)]
    // pub response_sender: mpsc::UnboundedSender<ChainResponse>,
    // #[behaviour(ignore)]
    // pub init_sender: mpsc::UnboundedSender<bool>,
    // #[behaviour(ignore)]
    // pub blockchain: Blockchain,
}

impl AppNetworkBehavior {
    pub async fn new(
        _blockchain: Blockchain,
        _response_sender: mpsc::UnboundedSender<ChainResponse>,
        _init_sender: mpsc::UnboundedSender<bool>,
    ) -> Self {
        let mut behaviour = Self {
            // blockchain,
            // floodsub: Floodsub::new(*PEER_ID),
            mdns: Mdns::new(Default::default())
                .await
                .expect("failed to create mdns"),
            // response_sender,
            // init_sender,
        };
        // behaviour.floodsub.subscribe(CHAIN_TOPIC.clone());
        // behaviour.floodsub.subscribe(BLOCK_TOPIC.clone());
        // behaviour.floodsub.subscribe(TXN_TOPIC.clone());

        behaviour
    }
}

// incoming event handler
// impl NetworkBehaviourEventProcess<FloodsubEvent> for AppBehaviour {
//     fn inject_event(&mut self, event: FloodsubEvent) {
//         if let FloodsubEvent::Message(msg) = event {
//             if let Ok(resp) = serde_json::from_slice::<ChainResponse>(&msg.data) {
//                 if resp.receiver == PEER_ID.to_string() {
//                     info!("Response from {}:", msg.source);
//                     // resp.blocks.iter().for_each(|r| info!("{:?}", r));

//                     self.blockchain.replace_chain(&resp.blocks);
//                     self.blockchain.mempool.transactions = resp
//                         .txns
//                         .into_iter()
//                         .filter(|txn| Transaction::verify_txn(txn).is_ok())
//                         .collect();
//                 }
//             } else if let Ok(resp) = serde_json::from_slice::<ChainRequest>(&msg.data) {
//                 info!(
//                     "sending local chain & mempool to {}",
//                     msg.source.to_string()
//                 );
//                 let peer_id = resp.from_peer_id;

//                 if PEER_ID.to_string() == peer_id {
//                     let json = serde_json::to_string(&ChainResponse {
//                         blocks: self.blockchain.chain.clone(),
//                         txns: self.blockchain.mempool.transactions.clone(),
//                         receiver: msg.source.to_string(),
//                     })
//                     .expect("can jsonify response");

//                     self.floodsub.publish(CHAIN_TOPIC.clone(), json.as_bytes());
//                 }
//             } else if let Ok(block) = serde_json::from_slice::<Block>(&msg.data) {
//                 // info!("received new block from {}", msg.source.to_string());
//                 info!("received new block {:?}", block);
//                 if self.blockchain.chain.last().unwrap().id < block.id
//                     && self.blockchain.is_valid_block(block.clone())
//                 {
//                     info!("relaying new valid block");
//                     let json = serde_json::to_string(&block).expect("can jsonify request");
//                     self.floodsub.publish(BLOCK_TOPIC.clone(), json.as_bytes());
//                 }
//             } else if let Ok(txn) = serde_json::from_slice::<Transaction>(&msg.data) {
//                 info!("received new transaction from {}", msg.source.to_string());

//                 if !self.blockchain.txn_exist(&txn) && Transaction::verify_txn(&txn).is_ok() {
//                     info!("relaying new valid transaction");
//                     let json = serde_json::to_string(&txn).expect("can jsonify request");
//                     self.floodsub.publish(TXN_TOPIC.clone(), json.as_bytes());
//                     self.blockchain.add_txn(txn);
//                 }
//             }
//         }
//     }
// }

impl NetworkBehaviourEventProcess<MdnsEvent> for AppNetworkBehavior {
    fn inject_event(&mut self, event: MdnsEvent) {
        // match event {
        //     MdnsEvent::Discovered(discovered_list) => {
        //         for (peer, _addr) in discovered_list {
        //             self.floodsub.add_node_to_partial_view(peer);
        //         }
        //     }
        //     MdnsEvent::Expired(expired_list) => {
        //         for (peer, _addr) in expired_list {
        //             if !self.mdns.has_node(&peer) {
        //                 self.floodsub.remove_node_from_partial_view(&peer);
        //             }
        //         }
        //     }
        // }
    }
}
