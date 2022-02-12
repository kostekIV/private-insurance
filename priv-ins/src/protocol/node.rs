use crate::crypto::shares::{sum_elems, BeaverShare, Elem, Share, Shares, Beaver};
use crate::protocol::{Alpha, CirId, NodeCommands, NodeEvents, NodeId, sub_id, VarId};
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
use crate::protocol::node::NodeState::{HaveBeaver, HaveShares, Proceed, WaitForBeaver, WaitForShares, WaitForVariable};


enum NodeState {
    Proceed,
    WaitForVariable(CirId),
    WaitForBeaver(CirId, Share, Share),
    WaitForShares(CirId, CirId, CirId, Share, Share, Share),
    HaveBeaver(CirId, Share, Share),
    HaveShares(CirId, CirId, CirId, Share, Share, Share),
}

pub struct Node {
    id: NodeId,
    alpha_channel: Receiver<Alpha>,
    party_commands: Sender<NodeCommands>,
    party_events: Receiver<NodeEvents>,
    evaluated: HashMap<CirId, Share>,
    fully_open: HashMap<CirId, Shares>,
    variables: HashMap<CirId, Elem>,
    beavers: HashMap<CirId, BeaverShare>,
    variable_shares: HashMap<CirId, Share>,
    variable_salts: HashMap<CirId, Elem>,
}

impl Node {
    /// checks if we have both x - r and [r] for variable under `var_node` if so put x - r + [r] under
    /// var_node in evaluated nodes.
    fn combine_variable_if_full(&mut self, var_node: CirId, calculator: &Calculator) {
        if !self.variable_salts.contains_key(&var_node)
            || !self.variable_shares.contains_key(&var_node)
        {
            return;
        }

        let s1 = self.variable_shares.remove(&var_node).expect("checked");
        let s2 = self.variable_salts.remove(&var_node).expect("checked");

        self.evaluated.insert(var_node, calculator.add_const(s1, s2));
    }

    async fn wait_for_calculator(&mut self) -> Calculator {
        let Alpha(alpha) = self.alpha_channel
            .recv()
            .await
            .expect("Without alpha we are doomed anyway");

        Calculator::new(self.id, alpha)
    }


    fn can_proceed(&self, state: &NodeState) -> bool {
        match state {
            Proceed => { true }
            _ => false
        }
    }


    fn try_proceed(&mut self, calculator: &Calculator, evaluating: &MidEvalExpression) -> NodeState {
        match evaluating {
            MidEvalExpression::AddConstant(s, evaluated_node, cir_id) => {
                let evaluated = self
                    .evaluated
                    .remove(evaluated_node)
                    .expect("we should have already evaluated it");

                let v = calculator.add_const(evaluated, Elem::from(s.clone()));

                self.evaluated.insert(cir_id.to_string(), v);
            }
            MidEvalExpression::Add(e1, e2, cir_id) => {
                let ev1 = self
                    .evaluated
                    .remove(e1)
                    .expect("we should have already evaluated it");
                let ev2 = self
                    .evaluated
                    .remove(e2)
                    .expect("we should have already evaluated it");

                let v = calculator.add(ev1, ev2);

                self.evaluated.insert(cir_id.to_string(), v);
            }
            MidEvalExpression::MulConstant(s, evaluated_node, cir_id) => {
                let evaluated = self
                    .evaluated
                    .remove(evaluated_node)
                    .expect("we should have already evaluated it");

                let v = calculator.mul_by_const(evaluated, Elem::from(s.clone()));

                self.evaluated.insert(cir_id.to_string(), v);
            }
            MidEvalExpression::Mul(e1, e2, cir_id) => {
                let ev1 = self
                    .evaluated
                    .remove(e1)
                    .expect("we should have already evaluated it");
                let ev2 = self
                    .evaluated
                    .remove(e2)
                    .expect("we should have already evaluated it");

                return WaitForBeaver(cir_id.to_string(), ev1, ev2);
            }
            MidEvalExpression::Var(cir_id) => {
                if !self.evaluated.contains_key(cir_id) {
                    return WaitForVariable(cir_id.to_string())
                }
            }
            _ => {}
        }

        Proceed
    }


    async fn handle_beaver(&mut self, calculator: &Calculator, cir_id: CirId, ev1: Share, ev2: Share) -> NodeState {
        let beaver = self.beavers.remove(&cir_id).expect("checked");

        let (e, f) = calculator.mul_prepare(Share::from(ev1.clone()), Share::from(ev2.clone()), BeaverShare::from(beaver.clone()));

        let e_id = sub_id(&cir_id, &"e".to_string());
        let f_id = sub_id(&cir_id, &"f".to_string());

        self.party_commands.send(
            NodeCommands::OpenShare(e, e_id.to_string())
        ).expect("Send should succeed");

        self.party_commands.send(
            NodeCommands::OpenShare(f, f_id.to_string())
        ).expect("Send should succeed");


        WaitForShares(cir_id, e_id, f_id, ev1, ev2, beaver.2)
    }


    fn handle_shares(&mut self, calculator: &Calculator, cir_id: CirId, e_id: CirId, f_id: CirId, ev1: Share, ev2: Share, beaver_c: Share) -> NodeState {
        let e_shares = self.fully_open.remove(&e_id).expect("checked");
        let f_shares = self.fully_open.remove(&f_id).expect("checked");

        let e_elem = sum_elems(&e_shares.into_iter().map(|(e, _)| e).collect());
        let f_elem = sum_elems(&f_shares.into_iter().map(|(e, _)| e).collect());

        let v = calculator.mul(ev1, ev2, e_elem, f_elem, beaver_c);
        self.evaluated.insert(cir_id, v);

        Proceed
    }


    pub async fn run(mut self, exp: DecoratedExpression) {
        self.party_commands
            .send(NodeCommands::NeedAlphaFor(self.id));

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


        let mut state = Proceed;

        let circuit_nodes = exp.into_ordered();
        let mut idx = 1;

        loop {
            // everything evaluated
            if idx == circuit_nodes.len() {
                break;
            }

            state = match state {
                WaitForVariable(cir_id) => {
                    if self.evaluated.contains_key(&cir_id) {
                        Proceed
                    } else {
                        WaitForVariable(cir_id)
                    }
                }
                WaitForBeaver(cir_id, s1, s2) => {
                    if self.beavers.contains_key(&cir_id) {
                        HaveBeaver(cir_id, s1, s2)
                    } else {
                        WaitForBeaver(cir_id, s1, s2)
                    }
                },
                WaitForShares(c, e, f, s1, s2, beaver_c) => {
                    if self.fully_open.contains_key(&e) && self.fully_open.contains_key(&f) {
                        HaveShares(c, e, f, s1, s2, beaver_c)
                    } else {
                        WaitForShares(c, e, f, s1, s2, beaver_c)
                    }
                },
                state => {
                    state
                }
            };

            if self.can_proceed(&state) {
                let evaluating = circuit_nodes.get(idx).expect("we control it");
                state = self.try_proceed(&calculator, evaluating);
                idx += 1;
                continue;
            }

            state = match state {
                HaveBeaver(cir_id, ev1, ev2) => {
                    self.handle_beaver(&calculator, cir_id, ev1, ev2).await
                },
                HaveShares(cir_id, e_id, f_id, s1, s2, beaver_c) => {
                    self.handle_shares(&calculator, cir_id, e_id, f_id, s1, s2, beaver_c)
                }
                s => s
            };

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

                    self.fully_open.insert(c_id, s);
                }
                NodeEvents::SelfVariableReady(c_id, r, r_share) => {
                    if !self.variables.contains_key(&c_id) {
                        log::debug!("got foreign variable");
                        continue;
                    }
                    let x = self.variables.get(&c_id).expect("checked");

                    let xr = x.sub(r);
                    // send to everyone x-r
                    self.party_commands
                        .send(NodeCommands::OpenSelfShare(xr, c_id.clone()))
                        .expect("send should succeed");
                    // evaluate our variable as (x-r) + r_share
                    self.evaluated.insert(c_id, calculator.add_const(r_share, xr));
                }
                NodeEvents::NodeVariableReady(c_id, s) => {
                    if !self.variable_salts.contains_key(&c_id) {
                        log::debug!("got twice value for {}", c_id);
                        return;
                    }

                    self.variable_salts.insert(c_id.clone(), s);
                    self.combine_variable_if_full(c_id, &calculator);
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
                    self.combine_variable_if_full(c_id, &calculator);
                }
            }
        }
    }
}
