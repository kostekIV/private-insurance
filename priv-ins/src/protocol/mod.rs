mod arithmetics;
pub mod dealer;
mod expression;
pub mod network;
pub mod node;
pub mod party;
mod test;

use async_std::task;
use ff::{Field, PrimeField};
use std::collections::HashMap;

use crate::crypto::shares::{BeaverShare, Commitment, CommitmentProof, Elem, Share, Shares};
use crate::expressions::Expression;
use crate::protocol::{
    expression::decorate_expression, network::Network, node::Node, party::Party,
};
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
    /// Node needs its alpha
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
    /// Node needs its alpha
    NeedAlpha,
    /// Broadcast commitment for cir_id
    CommitmentFor(CirId, Commitment),
    /// Broadcast proof for cir_id
    ProofFor(CirId, CommitmentProof),
    /// Broadcast that proof was verified for cir_id
    ProofVerified(CirId),
    /// broadcast that proof was invalid
    ProofInvalid(CirId),
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
    /// got all commitments for cir_id
    CommitmentsFor(CirId, Vec<(NodeId, Commitment)>),
    /// got all proofs for cir_id
    ProofsFor(CirId, Vec<(NodeId, CommitmentProof)>),
    /// proof was verified for cir_id
    ProofValid(CirId),
    /// broadcast that proof was invalid
    ProofInvalid(CirId),
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
    pub id: NodeId,
    pub n_parties: u8,
    pub network: N,
    pub dealer: (Sender<(NodeId, DealerCommands)>, Receiver<DealerEvents>),
    pub expression: Expression<u64>,
    pub variables: HashMap<String, NodeId>,
    pub our_variables: HashMap<String, u64>,
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

    let node = Node::new(id, alpha_rx, node_cmd_tx, node_events_rx, variables);
    let mut party = Party::new(
        id,
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

    let node_handle = task::spawn(node_task);
    task::spawn(party_task);

    let res = node_handle.await;
    println!("node {} finished with {:?}", id, res);
    tide::log::debug!("node {} finished with {:?}", id, res);
}
