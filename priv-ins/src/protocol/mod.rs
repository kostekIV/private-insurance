mod arithmetics;
mod dealer;
mod expression;
mod network;
mod node;
mod party;
mod test;

use async_recursion::async_recursion;
use ff::{Field, PrimeField};
use std::collections::HashMap;
use std::iter;
use std::ops::Mul;

use crate::crypto::shares::{BeaverShare, Elem, Share, Shares};
use crate::expressions::{BinaryOp, Expression};
use crate::protocol::expression::decorate_expression;
use crate::protocol::network::Network;
use crate::protocol::node::Node;
use crate::protocol::party::Party;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{
    unbounded_channel, UnboundedReceiver as Receiver, UnboundedSender as Sender,
};

pub type NodeId = u64;
pub type VarId = String;

/// Id of node in the circuit
pub type CirId = String;

pub fn sub_id(id: &CirId, name: &CirId) -> CirId {
    format!("{}-{}", id, name)
}

#[derive(Debug)]
pub enum DealerEvents {
    /// sends r and [r] for sharing secret value `varid` of node
    /// node receiving this message should own the variable
    NodeSelfVariable(CirId, Elem, Share),
    /// sends share [r] for secret value `varid`
    NodeVariableShared(CirId, Share),
    /// sends beaver shares for cirid for this node.
    BeaverSharesFor(CirId, BeaverShare),
    /// gives alpha_i, and vec of (var_id, alpha_i * x_j)
    Alpha(Elem),
}

#[derive(Debug)]
pub enum DealerCommands {
    /// Node wants to secretly share its variable
    NodeOpenSelfInput(CirId),
    /// Node needs beaver for cir_id
    BeaverFor(CirId),
    NeedAlpha,
}

#[derive(Debug)]
pub enum NodeCommands {
    /// Node opens its share for CirId
    OpenShare(Share, CirId),
    /// Node opens its (share - r) for CirId
    OpenSelfShare(Elem, CirId),
    /// Node wants to secretly share its variable
    OpenSelfInput(CirId),
    /// Node needs beaver for cir_id
    NeedBeaver(CirId),
    NeedAlpha,
}

#[derive(Debug)]
pub enum NodeEvents {
    /// cir is ready with shares from all of nodes
    CirReady(CirId, Shares),
    /// parts for sharing variable `var_id` are ready (r, [r])
    SelfVariableReady(CirId, Elem, Share),
    /// (x - share) for var_id is ready
    NodeVariableReady(CirId, Elem),
    /// [share] for var_id is ready
    NodeVariableShareReady(CirId, Share),
    /// beaver for node in circuit is ready
    BeaverFor(CirId, BeaverShare),
}

#[derive(Debug)]
pub struct Alpha(Elem);

pub struct Provider {
    id: u64,
    var_to_node: HashMap<String, NodeId>,
}

impl Provider {
    pub fn from(var_to_node: HashMap<String, NodeId>) -> Self {
        Self { id: 0, var_to_node }
    }

    pub fn next(&mut self) -> CirId {
        self.id += 1;

        self.id.to_string()
    }

    pub fn var_to_node(&self, name: String) -> Option<NodeId> {
        self.var_to_node.get(&name).cloned()
    }
}

pub struct NodeConfig<N: Network> {
    id: NodeId,
    n_parties: u8,
    network: N,
    dealer: (Sender<DealerCommands>, Receiver<DealerEvents>),
    expression: Expression<u64>,
    variables: HashMap<String, NodeId>,
    our_variables: HashMap<String, u64>,
}

pub async fn run_node<N: Network + 'static + Send>(config: NodeConfig<N>) {
    let NodeConfig {
        id,
        n_parties,
        network,
        dealer,
        expression,
        variables,
        our_variables,
    } = config;

    let (node_cmd_tx, node_cmd_rx) = unbounded_channel();
    let (node_events_tx, node_events_rx) = unbounded_channel();
    let (alpha_tx, alpha_rx) = unbounded_channel();

    let mut provider = Provider {
        id: 0,
        var_to_node: variables,
    };

    let decorated = decorate_expression(expression, &mut provider).expect("");

    let mut variables = HashMap::new();
    for (cir_id, var_id) in decorated.self_var_ids(Some(id)) {
        variables.insert(cir_id, Elem::from(*our_variables.get(&var_id).expect("")));
    }

    let mut node = Node::new(id, alpha_rx, node_cmd_tx, node_events_rx, variables);
    let mut party = Party::new(
        dealer,
        alpha_tx,
        node_cmd_rx,
        node_events_tx,
        network,
        n_parties,
    );

    let node_task = async move {
        node.run(decorated).await;
    };
    let party_task = async move {
        party.run().await;
    };

    let node_handle = tokio::spawn(node_task);
    tokio::spawn(party_task);

    node_handle.await;
    log::debug!("node {} finished", id);
}
