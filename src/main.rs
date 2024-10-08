#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(non_camel_case_types)]
#![allow(clippy::four_forward_slashes)]

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
use util::peer_id_short;
use std::{process::exit, time::Duration};
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    select, spawn,
    sync::mpsc,
    time::sleep,
};

mod account;
mod block;
mod blockchain;
mod mempool;
mod p2p;
mod stake;
mod transaction;
mod util;
// mod validator;
mod wallet;

use blockchain::Blockchain;

use crate::wallet::Wallet;

// #[tokio::main] is a proc macro that essentially wraps the content of main() 
// into an async block, and then starts a Tokio runtime to spawn it.
// fn main() {
//     tokio::runtime::Builder::new_multi_thread()
//         .enable_all().build().unwrap()
//         .block_on(async {
//             // ... content of "async fn main() {} "
//         })
// }
#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let peer_id_short = peer_id_short(&p2p::PEER_ID);
    println!(""); println!("");
    info!("Peer Id Short: {:?}", peer_id_short);
    info!("Peer Id: {:?}", p2p::PEER_ID.to_string()); // ed25519 pub key

    let (response_sender, mut response_rcv) = mpsc::unbounded_channel();
    let (init_sender, mut init_rcv) = mpsc::unbounded_channel();
    let (pos_mining_sender, mut pos_mining_rcv) = mpsc::unbounded_channel::<bool>();

    let auth_keys = Keypair::<X25519Spec>::new().into_authentic(&p2p::KEYS)
        .expect("failed to create auth keys");

    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();


    let args: Vec<String> = std::env::args().collect();
    let wallet;
    if args.len() == 1 {
        wallet = Wallet::get_wallet("5ae5066dd048ffb8f8628c44324e63c7b8782a026009a85a96935acb4921abbc5aede624154386ca358af195e13a46981b917ee8279f30a67d7a211a3d3e7243".to_string());
    } else {
        wallet = Wallet::get_wallet("27a23bf39574e86464f4e638241b3ef3dd223d9a30bd97810ff29c992e747e5a230681c76f00b412ccf7757a8449c448a04acd735e497a7612b66d8bfcb8e576".to_string());
    }
    wallet.print();

    let behaviour = p2p::AppBehaviour::new(
        Blockchain::new(wallet),
        response_sender,
        init_sender.clone(),
    ).await;

    let mut swarm = SwarmBuilder::new(transp, behaviour, *p2p::PEER_ID)
        .executor(Box::new(|fut| { spawn(fut); }))
        .build();

    let mut stdin = BufReader::new(stdin()).lines();

    Swarm::listen_on(
        &mut swarm,
        "/ip4/0.0.0.0/tcp/0".parse().expect("failed to get a local socket"),
    ).expect("failed to listen on swarm");

    spawn(async move {
        sleep(Duration::from_secs(1)).await;
        info!("sending init event after 1 sec");
        init_sender.send(true).expect("failed to send init event");
    });

    // // Run mining_planner n seconds periodically
    // let seconds = 5;
    // let mut mining_planner = periodic::Planner::new();
    // mining_planner.start();
    // mining_planner.add(
    //     move || pos_mining_sender.send(true).expect("failed to send mine event"),
    //     periodic::Every::new(Duration::from_secs(seconds)),
    // );

    let mut cnt = 0;
    loop {
        println!("\n\n### main loop [{cnt}], p_id: {peer_id_short}");
        cnt += 1;
        let evt = {
            select! {
                line = stdin.next_line() => Some(p2p::EventType::Input(line.expect("failed to get line").expect("can read line from stdin"))),
                _init = init_rcv.recv() => {
                    println!("-- select: 'init' received");
                    Some(p2p::EventType::Init)
                }
                _ = pos_mining_rcv.recv() => {
                    println!("-- select: 'mine' received" );
                    Some(p2p::EventType::Mining)
                },
                _ = swarm.select_next_some() => {
                    println!("-- select: next");
                    // info!("Unhandled Swarm Event: {:?}", event);
                    None
                },
            }
        };

        if let Some(event) = evt {
            match event {
                p2p::EventType::Init => {
                    println!("-- initing ..");
                    let peers = p2p::get_list_peers(&swarm);
                    info!("connected nodes: {}", peers.len());
                    if !peers.is_empty() {
                        let req = p2p::ChainRequest {
                            from_peer_id: peers.iter().last()
                                .expect("failed to get at least one peer")
                                .to_string(),
                        };
                        let json = serde_json::to_string(&req).expect("failed to jsonify request");
                        info!("ChainRequest to peer *{}", &req.from_peer_id[52-4..]);
                        swarm.behaviour_mut()
                             .floodsub
                             .publish(p2p::CHAIN_TOPIC.clone(), json.as_bytes());
                    }
                }
                p2p::EventType::Mining => {
                    println!(">> mining");
                    if let Some(block) 
                            = swarm.behaviour_mut().blockchain.mine_block_by_stake() 
                    {
                        swarm.behaviour_mut().blockchain.add_new_block(block.clone());
                        info!("broadcasting new block");
                        let json = serde_json::to_string(&block)
                                        .expect("failed to jsonify request");
                        swarm.behaviour_mut()
                            .floodsub
                            .publish(p2p::BLOCK_TOPIC.clone(), json.as_bytes());
                    };
                }
                p2p::EventType::Input(line) => match line.as_str() {
                    "ls p" => p2p::handle_print_peers(&swarm),
                    "mine" => pos_mining_sender.send(true).expect("failed to send mine event"),
                    // "create wallet" => Wallet::generate_wallet(),
                    // "ls wallet" => p2p::handle_print_wallet(&mut swarm),
                    // "ls c" => p2p::handle_print_chain(&swarm),
                    "bal" => p2p::handle_print_balance(&swarm),
                    // "ls validator" => p2p::handle_print_validator(&swarm),
                    // "ls stakes" => p2p::handle_print_stake(&swarm),
                    "mp" => p2p::handle_print_mempool(&swarm),
                    // cmd if cmd.starts_with("set wallet") => p2p::handle_set_wallet(cmd, &mut swarm),
                    cmd if cmd.starts_with("dd") => p2p::dummy_txn(cmd, &mut swarm),                    
                    // cmd if cmd.starts_with("new txn") => p2p::handle_create_txn(cmd, &mut swarm),
                    _ => error!("unknown command"),
                },
                _ => todo!()
            }
        }
    }
}
