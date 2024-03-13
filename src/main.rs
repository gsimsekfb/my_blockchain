#![allow(unused_imports)]

use libp2p::{
    core::upgrade,
    futures::StreamExt,
    mplex,
    noise::{Keypair, NoiseConfig, X25519Spec},
    swarm::{Swarm, SwarmBuilder},
    tcp::TokioTcpConfig,
    Transport,
};
use log::{error, info, warn};
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    select, spawn,
    sync::mpsc,
    time::sleep,
};

mod block;
mod blockchain;
mod p2p;
mod transaction;
mod wallet;

use wallet::Wallet;
use blockchain::Blockchain;

async fn init() {
    let (init_tx, mut _init_rx) = tokio::sync::mpsc::unbounded_channel();
    let (response_tx, mut _response_rx) = mpsc::unbounded_channel();
    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&p2p::KEYS)
        .expect("failed to create auth keys");
    // Create a new configuration for a TCP/IP transport:
    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();
    let wallet = Wallet::new();
    let behaviour = p2p::AppBehaviour::new(
        Blockchain::new(wallet),
        response_tx,
        init_tx.clone(),
    ).await;

    let mut swarm = SwarmBuilder::new(transp, behaviour, *p2p::PEER_ID)
        .executor(Box::new(|fut| {
            spawn(fut);
        }))
        .build();

    Swarm::listen_on(
        &mut swarm,
        "/ip4/0.0.0.0/tcp/0".parse().expect("failed to get a local socket"),
    ).expect("swarm cannot be started");

    let peers = p2p::get_list_peers(&swarm);
    println!("peers.len(): {}", peers.len());
    info!("connected nodes: {}", peers.len());
}

// #[tokio::main] is a proc macro that essentially wraps the content of main() 
// into an async block, and then starts a Tokio runtime to spawn it.
// #[tokio::main]
// async fn main() {
//     println!("hello");
// }
// ... gets transformed into:
// fn main() {
//     tokio::runtime::Builder::new_multi_thread()
//         .enable_all().build().unwrap()
//         .block_on(async {
//             println!("hello");
//         })
// }
#[tokio::main]
async fn main() {
    println!("Hello, world!");
    pretty_env_logger::init();

    init().await;

    // tokio::spawn(async move {
    //     tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    //     info!("sending init event");
    //     init_sender.send(true).expect("can send init event");
    // });

    // while let Some(res) = init_rx.recv().await {
    //     println!("got = {}", res);
    // }
}