#[macro_use]
extern crate ff;
extern crate futures;
extern crate tokio;

use crate::rest::expression;
use std::{collections::HashMap, fs};
use tide::http::headers::HeaderValue;
use tide::log::LevelFilter;

mod crypto;
mod expressions;
mod protocol;
mod rest;

use crate::expressions::BinaryOp::{Add, Mul};
use crate::expressions::Expression;
use crate::protocol::dealer::TrustedDealer;
use crate::protocol::network::setup_network;
use crate::protocol::{run_node, NodeConfig};
use serde::Deserialize;
use tide::security::{CorsMiddleware, Origin};
use tokio::sync::mpsc::unbounded_channel;

fn get_cors() -> CorsMiddleware {
    CorsMiddleware::new()
        .allow_methods("GET, POST, OPTIONS".parse::<HeaderValue>().unwrap())
        .allow_origin(Origin::from("*"))
        .allow_credentials(false)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct VariableConfig {
    nodes: Vec<HashMap<String, u64>>,
}

#[tokio::main]
async fn main() {
    run_nodes().await;
}

async fn run_nodes() {
    let n_parties = 5;

    let networks = setup_network(n_parties);
    let (senders, receivers): (Vec<_>, Vec<_>) =
        (0..n_parties).map(|_| unbounded_channel()).unzip();
    let (cmd_tx, cmd_rx) = unbounded_channel();

    let dealer = TrustedDealer::new(
        n_parties as u8,
        senders
            .into_iter()
            .enumerate()
            .map(|(i, s)| (i as u64, s))
            .collect(),
        cmd_rx,
    );

    let variable_config: VariableConfig = serde_json::from_str(
        &fs::read_to_string("variables_config.json")
            .expect("Unable to read config file containing peer addresses"),
    )
    .expect("JSON was not well-formatted");
    println!("{:?}", variable_config);

    let mut handles = vec![];
    let d = tokio::spawn(dealer.run());
    for ((id, n), r) in (0..n_parties)
        .zip(networks.into_iter())
        .zip(receivers.into_iter())
    {
        let expression = Expression::<u64>::BinOp {
            left: Box::new(Expression::<u64>::BinOp {
                left: Box::new(Expression::<u64>::BinOp {
                    left: Box::new(Expression::<u64>::BinOp {
                        left: Box::new(Expression::<u64>::BinOp {
                            left: Box::new(Expression::Number { number: 10 }),
                            right: Box::new(Expression::Variable {
                                name: "0".to_string(),
                            }),
                            op: Mul,
                        }),
                        right: Box::new(Expression::Variable {
                            name: "1".to_string(),
                        }),
                        op: Mul,
                    }),
                    right: Box::new(Expression::Variable {
                        name: "2".to_string(),
                    }),
                    op: Mul,
                }),
                right: Box::new(Expression::Variable {
                    name: "3".to_string(),
                }),
                op: Mul,
            }),
            right: Box::new(Expression::Variable {
                name: "4".to_string(),
            }),
            op: Add,
        };
        let variables = (0..n_parties)
            .map(|id| (id.to_string(), id as u64))
            .collect();
        let our_variables = variable_config.nodes[id as usize].clone();
        let config = NodeConfig {
            id: id as u64,
            n_parties: n_parties as u8,
            network: n,
            dealer: (cmd_tx.clone(), r),
            expression,
            variables,
            our_variables,
        };
        handles.push(tokio::spawn(run_node(config)));
    }

    for handle in handles {
        handle.await.expect("node works :)");
    }
}
