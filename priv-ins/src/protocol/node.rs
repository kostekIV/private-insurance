use crate::crypto::shares::{sum_shares, BeaverShare, Elem, Share, Shares};
use crate::protocol::{Alpha, CirId, NodeCommands, NodeEvents, NodeId, VarId};
use async_recursion::async_recursion;
use std::collections::HashMap;
use std::ops::Sub;

use crate::protocol::arithmetics::Calculator;
use crate::protocol::expression::{DecoratedExpression, MidEvalExpression};
use futures::prelude::*;
use tokio::select;
use tokio::sync::mpsc::{
    unbounded_channel, UnboundedReceiver as Receiver, UnboundedSender as Sender,
};

pub struct Node {
    id: NodeId,
    alpha_channel: Receiver<Alpha>,
    party_commands: Sender<NodeCommands>,
    party_events: Receiver<NodeEvents>,
    evaluated: HashMap<CirId, Share>,
    fully_open: HashMap<CirId, Share>,
    variables: HashMap<CirId, Elem>,
    beavers: HashMap<CirId, BeaverShare>,
    variable_shares: HashMap<CirId, Share>,
    variable_salts: HashMap<CirId, Elem>,
}

impl Node {
    /// checks if we have both x - r and [r] for variable under `var_node` if so put x - r + [r] under
    /// var_node in evaluated nodes.
    fn combine_variable_if_full(&mut self, var_node: CirId) {
        if !self.variable_salts.contains_key(&var_node)
            || !self.variable_shares.contains_key(&var_node)
        {
            return;
        }

        let s1 = self.variable_shares.remove(&var_node).expect("checked");
        let s2 = self.variable_salts.remove(&var_node).expect("checked");

        // self.evaluated.insert(var_node, s1 + s2);
    }

    async fn wait_for_calculator(&mut self) -> Calculator {
        let Alpha(alpha) = self.alpha_channel
            .recv()
            .await
            .expect("Without alpha we are doomed anyway");

        Calculator::new(self.id, alpha)
    }


    pub async fn run(mut self, exp: DecoratedExpression) {
        self.party_commands
            .send(NodeCommands::NeedAlpha);

        // announce need for beaver for this circuit nodes
        for mul_id in exp.mul_ids() {
            self.party_commands
                .send(NodeCommands::NeedBeaver(mul_id))
                .expect("send should succeed");
        }

        // announce to delear our variable
        for var_id in exp.self_var_ids(Some(self.id)) {
            self.party_commands
                .send(NodeCommands::OpenSelfInput(var_id))
                .expect("send should succeed");
        }

        let calculator = self.wait_for_calculator().await;

        let circuit_nodes = exp.into_ordered();
        let mut idx = 0;

        loop {
            // everything evaluated
            if idx == circuit_nodes.len() {
                break;
            }

            let evaluating = circuit_nodes.get(idx).expect("we control it");

            match evaluating {
                MidEvalExpression::AddConstant(s, evaluated_node, cir_id) => {
                    let evaluated = self
                        .evaluated
                        .get(evaluated_node)
                        .expect("we should have already evaluated it");

                    // addconst here;
                    // add to evaluated
                    idx += 1;
                    continue;
                }
                MidEvalExpression::Add(e1, e2, cir_id) => {
                    let ev1 = self
                        .evaluated
                        .get(e1)
                        .expect("we should have already evaluated it");
                    let ev2 = self
                        .evaluated
                        .get(e2)
                        .expect("we should have already evaluated it");

                    // add here;
                    // add to evaluated
                    idx += 1;
                    continue;
                }
                MidEvalExpression::MulConstant(s, evaluated_node, cir_id) => {
                    let evaluated = self
                        .evaluated
                        .get(evaluated_node)
                        .expect("we should have already evaluated it");

                    // mulconst here;
                    // add to evaluated
                    idx += 1;
                    continue;
                }
                MidEvalExpression::Mul(e1, e2, cir_id) => {
                    let ev1 = self
                        .evaluated
                        .get(e1)
                        .expect("we should have already evaluated it");
                    let ev2 = self
                        .evaluated
                        .get(e2)
                        .expect("we should have already evaluated it");

                    // mul here;
                    // add to evaluated
                    idx += 1;
                    continue;
                }
                MidEvalExpression::Var(cir_id) => {
                    if self.evaluated.contains_key(cir_id) {
                        idx += 1;
                        continue;
                    }
                }
                _ => {}
            }

            let event = match self.party_events.recv().await {
                Some(e) => e,
                None => {
                    log::debug!("party channel closed");
                    return;
                }
            };

            match event {
                NodeEvents::CirReady(c_id, s) => {
                    // evaluate node as sum of gotten shares
                    if self.fully_open.contains_key(&c_id) {
                        log::debug!("got twice opened value for {}", c_id);
                    }

                    // self.fully_open.insert(c_id, sum_shares(&s));
                }
                NodeEvents::SelfVariableReady(c_id, r, r_share) => {
                    if !self.variables.contains_key(&c_id) {
                        log::debug!("got foreign variable");
                        continue;
                    }
                    let x = self.variables.get(&c_id).expect("checked");

                    // let xr = x.sub(r);
                    // // send to everyone x-r
                    // self.party_commands
                    //     .send(NodeCommands::OpenSelfShare(xr, c_id.clone()))
                    //     .expect("send should succeed");
                    // // evaluate our variable as (x-r) + r_share
                    // self.evaluated.insert(c_id, xr + r_share);
                }
                NodeEvents::NodeVariableReady(c_id, s) => {
                    if !self.variable_salts.contains_key(&c_id) {
                        log::debug!("got twice value for {}", c_id);
                        return;
                    }

                    self.variable_salts.insert(c_id.clone(), s);
                    self.combine_variable_if_full(c_id);
                }
                NodeEvents::BeaverFor(c_id, beaver) => {
                    if !self.beavers.contains_key(&c_id) {
                        log::debug!("got twice value for {}", c_id);
                    }

                    self.beavers.insert(c_id, beaver);
                }
                NodeEvents::NodeVariableShareReady(c_id, s) => {
                    if !self.variable_shares.contains_key(&c_id) {
                        log::debug!("got twice value for {}", c_id);
                        return;
                    }

                    self.variable_shares.insert(c_id.clone(), s);
                    self.combine_variable_if_full(c_id);
                }
            }
        }
    }
}
