mod dealer;
mod node;
mod party;
mod network;
mod test;

use std::collections::HashMap;
use ff::{Field, PrimeField};
use std::ops::{Mul};
use async_recursion::async_recursion;

use crate::crypto::shares::{BeaverShare, Share, Shares};
use crate::expressions::{BinaryOp, Expression};
use crate::protocol::DecoratedExpression::{Add, AddConstant, Constant, Mul as MulExpr, MulConstant, Var};

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
    NodeSelfVariable(VarId, Share, Share),
    /// sends share [r] for secret value `varid`
    NodeVariableShared(VarId, Share),
    /// sends beaver shares for cirid for this node.
    BeaverSharesFor(CirId, BeaverShare)
}

#[derive(Debug)]
pub enum DealerCommands {
    /// Node wants to secretly share its variable
    NodeOpenSelfInput(VarId),
    /// Node needs beaver for cir_id
    BeaverFor(CirId)
}

#[derive(Debug)]
pub enum NodeCommands {
    /// Node opens its share for CirId
    OpenShare(Share, CirId),
    /// Node wants to secretly share its variable
    OpenSelfInput(VarId),
    /// Node needs beaver for cir_id
    NeedBeaver(CirId),
}

#[derive(Debug)]
pub enum NodeEvents {
    /// cir is ready with shares from all of nodes
    CirReady(CirId, Shares),
    /// parts for sharing variable `var_id` are ready (r, [r])
    SelfVariableReady(VarId, Share, Share),
    /// share for var_id is ready
    NodeVariableReady(VarId, Share),
    /// beaver for node in circuit is ready
    BeaverFor(CirId, BeaverShare),
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
    AddConstant(Share, BExpression, CirId),
    Add(BExpression, BExpression, CirId),
    Mul(BExpression, BExpression, CirId),
    MulConstant(Share, BExpression, CirId),
    Var(NodeId, VarId, CirId),
    Constant(Share, CirId),
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



#[async_recursion]
pub async fn decorate_expression<D: Dealer + Send>(expr: Expression<u64>, id_provider: &mut Provider, dealer: &mut D) -> Result<DecoratedExpression, String> {
    match expr {
        Expression::Number { number } => {
            Ok(DecoratedExpression::Constant(Share::from(number), id_provider.next()))
        }
        Expression::BinOp { left, right, op } => {
            let left = decorate_expression(*left, id_provider, dealer).await?;
            let right = decorate_expression(*right, id_provider, dealer).await?;

            match op {
                BinaryOp::Add => {
                    match (left, right) {
                        (Constant(s1, _), Constant(s2,_)) => { Ok(Constant(s1 + s2, id_provider.next())) }
                        (Constant(s1, _), x) => { Ok(AddConstant(s1, Box::new(x), id_provider.next())) }
                        (x, Constant(s1, _)) => { Ok(AddConstant(s1, Box::new(x), id_provider.next())) }
                        (x, y) => { Ok(Add(Box::new(x), Box::new(y), id_provider.next())) }
                    }
                }
                BinaryOp::Mul => {
                    match (left, right) {
                        (Constant(s1, _), Constant(s2, _)) => { Ok(Constant(s1 + s2, id_provider.next())) }
                        (Constant(s1, _), x) => { Ok(MulConstant(s1, Box::new(x),id_provider.next())) }
                        (x, Constant(s1, _)) => { Ok(MulConstant(s1, Box::new(x), id_provider.next())) }
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
            Ok(Var(node_id, name, id_provider.next()))
        }
    }
}
