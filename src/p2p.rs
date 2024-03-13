use crate::{
    block::Block, blockchain::Blockchain, transaction, transaction::Transaction,
    wallet::Wallet,
};
use libp2p::{
    floodsub::{Floodsub, FloodsubEvent, Topic},
    identity::Keypair,
    mdns::{Mdns, MdnsEvent},
    swarm::{NetworkBehaviourEventProcess, Swarm},
    NetworkBehaviour, PeerId,
};
use serde::{Deserialize, Serialize};
use once_cell::sync::Lazy;
use tokio::sync::mpsc;

pub static KEYS: Lazy<Keypair> = Lazy::new(Keypair::generate_ed25519);

// #[derive(Debug, Serialize, Deserialize)]
pub struct ChainResponse {
    pub blocks: Vec<Block>,
    pub txns: Vec<Transaction>,
    pub receiver: String,
}

// #[derive(NetworkBehaviour)]
pub struct AppBehaviour {
    // pub floodsub: Floodsub,
    // pub mdns: Mdns,
    // #[behaviour(ignore)]
    // pub response_sender: mpsc::UnboundedSender<ChainResponse>,
    // #[behaviour(ignore)]
    // pub init_sender: mpsc::UnboundedSender<bool>,
    // #[behaviour(ignore)]
    // pub blockchain: Blockchain,
}

impl AppBehaviour {
    pub async fn new(
        blockchain: Blockchain,
        response_sender: mpsc::UnboundedSender<ChainResponse>,
        init_sender: mpsc::UnboundedSender<bool>,
    ) -> Self {
        // let mut behaviour = Self {
        //     blockchain,
        //     floodsub: Floodsub::new(*PEER_ID),
        //     mdns: Mdns::new(Default::default())
        //         .await
        //         .expect("can create mdns"),
        //     response_sender,
        //     init_sender,
        // };
        // behaviour.floodsub.subscribe(CHAIN_TOPIC.clone());
        // behaviour.floodsub.subscribe(BLOCK_TOPIC.clone());
        // behaviour.floodsub.subscribe(TXN_TOPIC.clone());

        // behaviour

        Self {}
    }
}
