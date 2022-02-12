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
    AlphaFor(Elem),
}

#[derive(Debug)]
pub enum DealerCommands {
    /// Node wants to secretly share its variable
    NodeOpenSelfInput(VarId),
    /// Node needs beaver for cir_id
    BeaverFor(CirId),
    NeedAlphaFor(NodeId),
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
    NeedAlphaFor(NodeId),
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

#[async_trait::async_trait]
pub trait Dealer {
    /// Creates beaver share for `id`.
    async fn new_beaver(&mut self, id: &CirId);
    /// prepares variable to be secretly shared
    async fn prepare_variable(&mut self, nid: &NodeId, vid: &VarId);
}

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
