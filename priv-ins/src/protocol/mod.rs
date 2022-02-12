mod dealer;
mod node;
mod party;
mod network;
mod test;

use std::collections::HashMap;
use ff::{Field, PrimeField};
use std::ops::{Mul};
use async_recursion::async_recursion;

use crate::crypto::shares::{BeaverShare, Share};
use crate::expressions::{BinaryOp, Expression};
use crate::protocol::DecoratedExpression::{Add, AddConstant, Constant, Mul as MulExpr, MulConstant, Var};

pub type NodeId = u64;
pub type VarId = String;

/// Id of node in the circuit
pub type CirId = String;

pub fn sub_id(id: &CirId, name: &CirId) -> CirId {
    format!("{}-{}", id, name)
}

pub trait Connection {

}

#[async_trait::async_trait]
pub trait Party {
    /// Opens value under `id`, waits for every share to be delivered and returns discovered value
    async fn open(&mut self, id: &CirId, value: Share) -> Share;
    /// Retrieve beaver share for `id`.
    async fn beaver_for(&mut self, id: &CirId) -> BeaverShare;
    /// returns r, [r] for NodeID
    async fn open_self_input(&mut self, nid: NodeId, vid: VarId) -> (Share, Share);
    /// broadcast from nid about vid (which is hided as (x - r)
    async fn broadcast_self_input(&mut self, nid: NodeId, vid: VarId, share: Share);
    /// returns share for NodeId (x - r) + [r]
    async fn get_input_shares(&mut self, nid: NodeId, vid: VarId) -> Share;
    /// returns if node can add constant
    fn can_add(&self, nid: NodeId) -> bool;
}

#[async_trait::async_trait]
pub trait Dealer {
    /// Creates beaver share for `id`.
    async fn new_beaver(&mut self, id: &CirId);
    /// prepares variable to be secretly shared
    async fn prepare_variable(&mut self, nid: &NodeId, vid: &VarId);
}

type BExpression = Box<DecoratedExpression>;

pub enum DecoratedExpression {
    AddConstant(Share, BExpression),
    Add(BExpression, BExpression),
    Mul(BExpression, BExpression, CirId),
    MulConstant(Share, BExpression),
    Var(NodeId, VarId),
    Constant(Share),
}

pub struct Provider {
    id: u64,
    var_to_node: HashMap<String, NodeId>,
}

impl Provider {
    pub fn from(var_to_node: HashMap<String, NodeId>) -> Self {
        Self {
            id: 0,
            var_to_node,
        }
    }

    pub fn next(&mut self) -> CirId {
        self.id += 1;

        self.id.to_string()
    }

    pub fn var_to_node(&self, name: String) -> Option<NodeId> {
        self.var_to_node.get(&name).cloned()
    }
}


/// Multiply share s1 by constant c
pub fn mul_by_const(s1: &Share, c: &Share) -> Share {
    s1.mul(c)
}

/// todo doc
pub async fn mul<P: Party>(g_id: &CirId, s1: Share, s2: Share, party: &mut P) -> Share {
    let (a, b, c) = party.beaver_for(&g_id).await;

    let e = party.open(&sub_id(&g_id, &"e".to_string()), s1 - a).await;
    let d = party.open(&sub_id(&g_id, &"e".to_string()), s2 - b).await;

    c + mul_by_const(&b, &e) + mul_by_const(&a, &d) + e*d
}


#[async_recursion]
pub async fn decorate_expression<D: Dealer + Send>(expr: Expression<u64>, id_provider: &mut Provider, dealer: &mut D) -> Result<DecoratedExpression, String> {
    match expr {
        Expression::Number { number } => {
            Ok(DecoratedExpression::Constant(Share::from(number)))
        }
        Expression::BinOp { left, right, op } => {
            let left = decorate_expression(*left, id_provider, dealer).await?;
            let right = decorate_expression(*right, id_provider, dealer).await?;

            match op {
                BinaryOp::Add => {
                    match (left, right) {
                        (Constant(s1), Constant(s2)) => { Ok(Constant(s1 + s2)) }
                        (Constant(s1), x) => { Ok(AddConstant(s1, Box::new(x))) }
                        (x, Constant(s1)) => { Ok(AddConstant(s1, Box::new(x))) }
                        (x, y) => { Ok(Add(Box::new(x), Box::new(y))) }
                    }
                }
                BinaryOp::Mul => {
                    match (left, right) {
                        (Constant(s1), Constant(s2)) => { Ok(Constant(s1 + s2)) }
                        (Constant(s1), x) => { Ok(MulConstant(s1, Box::new(x))) }
                        (x, Constant(s1)) => { Ok(MulConstant(s1, Box::new(x))) }
                        (x, y) => {
                            let id = id_provider.next();
                            dealer.new_beaver(&id).await;
                            Ok(MulExpr(Box::new(x), Box::new(y), id))
                        }
                    }
                }
                _ => Err(format!("Only add and mul for now"))
            }
        }
        Expression::Variable { name } => {
            let node_id = id_provider.var_to_node(name.clone()).ok_or(format!("orphaned variable"))?;
            dealer.prepare_variable(&node_id, &name).await;
            Ok(Var(node_id, name))
        }
    }
}
