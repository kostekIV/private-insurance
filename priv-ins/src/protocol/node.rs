use crate::crypto::shares::{
    sum_elems, BeaverShare, Commitment, CommitmentProof, Elem, Share, Shares,
};
use crate::ff::PrimeField;
use std::{collections::HashMap, fmt::Debug, ops::Sub};

use crate::protocol::{
    arithmetics::Calculator,
    expression::{DecoratedExpression, MidEvalExpression},
    node::NodeState::{
        HaveBeaver, HaveShares, Proceed, WaitForBeaver, WaitForCommitments, WaitForShares,
        WaitForVariable,
    },
    sub_id, Alpha, CirId, NodeCommands, NodeEvents, NodeId,
};
use tokio::sync::mpsc::{UnboundedReceiver as Receiver, UnboundedSender as Sender};

#[derive(Debug)]
/// Internal state of node with following transitions:
/// Proceed -> WaitForVariable | WaitForBeaver | Proceed
/// WaitForVariable -> Proceed | WaitForVariable
/// WaitForBeaver -> HaveBeaver | WaitForBeaver
/// HaveBeaver -> WaitForShares
/// WaitForShares -> HaveShares | WaitForShare
/// HaveShares -> WaitForCommitments
/// WaitForCommitments -> Proceed | WaitForCommitments
///
/// In particular following path represents multiplication phases.
/// WaitForBeaver -> HaveBeaver -> WaitForShares -> HaveShares -> WaitForCommitments
enum NodeState {
    /// we can proceed with evaluating
    Proceed,
    /// waiting for shares of variable in the cir_id
    WaitForVariable(CirId),
    /// waiting for beaver shares in mul cir_id node
    WaitForBeaver(CirId, Share, Share),
    /// waiting for shares of (x - e) and (y - f) used in mul
    WaitForShares(CirId, CirId, CirId, BeaverShare),
    /// got beaver shares for mul cir_id node
    HaveBeaver(CirId, Share, Share),
    /// have all shares of (x - e) and (y - f) used in mul
    HaveShares(CirId, CirId, CirId, BeaverShare),
    /// wait for all commitments for e and f
    WaitForCommitments(CirId, CirId),
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
    my_proofs: HashMap<CirId, CommitmentProof>,
    commitments: HashMap<CirId, Vec<Commitment>>,
}

impl Node {
    pub fn new(
        id: NodeId,
        alpha_channel: Receiver<Alpha>,
        party_commands: Sender<NodeCommands>,
        party_events: Receiver<NodeEvents>,
        variables: HashMap<CirId, Elem>,
    ) -> Self {
        Self {
            id,
            alpha_channel,
            party_commands,
            party_events,
            variables,
            evaluated: HashMap::new(),
            fully_open: HashMap::new(),
            beavers: HashMap::new(),
            variable_shares: HashMap::new(),
            variable_salts: HashMap::new(),
            my_proofs: HashMap::new(),
            commitments: HashMap::new(),
        }
    }
    /// checks if we have both x - r and [r] for variable under `var_node` if so put x - r + [r] under
    /// var_node in evaluated nodes.
    fn combine_variable_if_full(&mut self, var_node: CirId, calculator: &Calculator) {
        if self.id == 0 {
            println!("combine_variable_if_full {:?}", var_node);
        }
        if !self.variable_salts.contains_key(&var_node)
            || !self.variable_shares.contains_key(&var_node)
        {
            return;
        }

        let s1 = self.variable_shares.remove(&var_node).expect("checked");
        let s2 = self.variable_salts.remove(&var_node).expect("checked");

        self.evaluated
            .insert(var_node, calculator.add_const(s1, s2));
    }

    async fn wait_for_calculator(&mut self) -> Calculator {
        if self.id == 0 {
            println!("wait_for_calculator");
        }

        let Alpha(alpha) = self
            .alpha_channel
            .recv()
            .await
            .expect("Without alpha we are doomed anyway");

        Calculator::new(self.id, alpha)
    }

    fn can_proceed(&self, state: &NodeState) -> bool {
        match state {
            Proceed => true,
            _ => false,
        }
    }

    fn try_proceed(
        &mut self,
        calculator: &Calculator,
        evaluating: &MidEvalExpression,
    ) -> NodeState {
        if self.id == 0 {
            println!("Evaluating MidEvalExpression::{:?}", evaluating);
        }

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
                    return WaitForVariable(cir_id.to_string());
                }
            }
        }

        Proceed
    }

    /// we got beaver lets start evaluating mul node
    async fn handle_beaver(
        &mut self,
        calculator: &Calculator,
        cir_id: CirId,
        ev1: Share,
        ev2: Share,
    ) -> NodeState {
        let beaver = self.beavers.remove(&cir_id).expect("checked");

        let (e, f) = calculator.mul_prepare(
            Share::from(ev1.clone()),
            Share::from(ev2.clone()),
            BeaverShare::from(beaver.clone()),
        );

        let e_id = sub_id(&cir_id, &"e".to_string());
        let f_id = sub_id(&cir_id, &"f".to_string());

        self.party_commands
            .send(NodeCommands::OpenShare(e, e_id.to_string()))
            .expect("Send should succeed");

        self.party_commands
            .send(NodeCommands::OpenShare(f, f_id.to_string()))
            .expect("Send should succeed");

        WaitForShares(cir_id, e_id, f_id, beaver)
    }

    /// we got all shares for mul nodes (of (x - e) and (y - f)
    fn handle_shares(
        &mut self,
        calculator: &Calculator,
        cir_id: CirId,
        e_id: CirId,
        f_id: CirId,
        beaver: BeaverShare,
    ) -> NodeState {
        let e_shares = self.fully_open.remove(&e_id).expect("checked");
        let f_shares = self.fully_open.remove(&f_id).expect("checked");

        let e_elem = sum_elems(&e_shares.into_iter().map(|(e, _)| e).collect());
        let f_elem = sum_elems(&f_shares.into_iter().map(|(e, _)| e).collect());

        let (e_hash, e_salt) = Calculator::generate_commitment(&e_elem);
        let (f_hash, f_salt) = Calculator::generate_commitment(&f_elem);

        let e_proof = (e_hash, e_elem.clone(), e_salt);
        let f_proof = (f_hash, f_elem.clone(), f_salt);

        self.my_proofs.insert(e_id.clone(), e_proof);
        self.my_proofs.insert(f_id.clone(), f_proof);

        self.party_commands
            .send(NodeCommands::CommitmentFor(e_id.clone(), e_hash))
            .expect("send should succeed");
        self.party_commands
            .send(NodeCommands::CommitmentFor(f_id.clone(), f_hash))
            .expect("send should succeed");

        let v = calculator.mul(beaver, e_elem, f_elem);
        self.evaluated.insert(cir_id, v);

        WaitForCommitments(e_id, f_id)
    }

    pub async fn run(mut self, exp: DecoratedExpression) {
        self.party_commands
            .send(NodeCommands::NeedAlpha)
            .expect("Send should succeed");

        // announce need for beaver for this circuit nodes
        for mul_id in exp.mul_ids() {
            self.party_commands
                .send(NodeCommands::NeedBeaver(mul_id))
                .expect("send should succeed");
        }

        // announce to dealer our variable
        for (var_id, _) in exp.self_var_ids(Some(self.id)) {
            self.party_commands
                .send(NodeCommands::OpenSelfInput(var_id))
                .expect("send should succeed");
        }

        let calculator = self.wait_for_calculator().await;

        let mut state = Proceed;

        let circuit_nodes = exp.into_ordered();
        let mut idx = 0;

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
                }
                WaitForShares(c, e, f, beaver) => {
                    if self.fully_open.contains_key(&e) && self.fully_open.contains_key(&f) {
                        HaveShares(c, e, f, beaver)
                    } else {
                        WaitForShares(c, e, f, beaver)
                    }
                }
                WaitForCommitments(c1, c2) => {
                    if self.commitments.contains_key(&c1) && self.commitments.contains_key(&c2) {
                        Proceed
                    } else {
                        WaitForCommitments(c1, c2)
                    }
                }
                state => state,
            };

            if self.id == 0 {
                println!("NodeState: {:?}", state);
            }

            if self.can_proceed(&state) {
                log::debug!("{}", idx);
                let evaluating = circuit_nodes.get(idx).expect("we control it");
                state = self.try_proceed(&calculator, evaluating);
                idx += 1;
                continue;
            }

            state = match state {
                HaveBeaver(cir_id, ev1, ev2) => {
                    self.handle_beaver(&calculator, cir_id, ev1, ev2).await
                }
                HaveShares(cir_id, e_id, f_id, beaver) => {
                    self.handle_shares(&calculator, cir_id, e_id, f_id, beaver)
                }
                s => s,
            };

            if self.can_proceed(&state) {
                idx += 1;
                continue;
            }

            let event = match self.party_events.recv().await {
                Some(e) => e,
                None => {
                    log::debug!("party channel closed");
                    return;
                }
            };

            if self.id == 0 {
                println!("NodeEvents::{:?}", event);
            }

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
                    self.evaluated
                        .insert(c_id, calculator.add_const(r_share, xr));
                }
                NodeEvents::NodeVariableReady(c_id, s) => {
                    if self.variable_salts.contains_key(&c_id) {
                        log::debug!("got twice value for {}", c_id);
                        continue;
                    }

                    self.variable_salts.insert(c_id.clone(), s);
                    self.combine_variable_if_full(c_id, &calculator);
                }
                NodeEvents::BeaverFor(c_id, beaver) => {
                    if self.beavers.contains_key(&c_id) {
                        log::debug!("got twice value for {}", c_id);
                        continue;
                    }

                    self.beavers.insert(c_id, beaver);
                }
                NodeEvents::NodeVariableShareReady(c_id, s) => {
                    if self.variable_shares.contains_key(&c_id) {
                        log::debug!("got twice value for {}", c_id);
                        continue;
                    }

                    self.variable_shares.insert(c_id.clone(), s);
                    self.combine_variable_if_full(c_id, &calculator);
                }
                NodeEvents::CommitmentsFor(cir_id, commitments) => {
                    if self.commitments.contains_key(&cir_id) {
                        log::debug!("got twice commitments for {}", cir_id);
                        continue;
                    }

                    self.commitments.insert(cir_id, commitments);
                }
            }
        }

        // we have final share lets open it now

        let last_node_id = circuit_nodes
            .last()
            .expect("at least one should exist")
            .cir_id();

        /// send all proofs
        /// wait for all proof
        /// checks proof
        /// continue
        /// annoce bad guy
        /// wait for all
        let evaluated = self
            .evaluated
            .remove(&last_node_id)
            .expect("we finished the evaluation");

        self.party_commands
            .send(NodeCommands::OpenShare(evaluated, last_node_id.clone()))
            .expect("should succeed");

        // now we wait for all shares :D
        loop {
            let event = match self.party_events.recv().await {
                Some(e) => e,
                None => {
                    log::debug!("party channel closed");
                    return;
                }
            };
            match event {
                NodeEvents::CirReady(c, s) => {
                    if c == last_node_id {
                        let el = sum_elems(&s.into_iter().map(|(e, _)| e).collect());

                        if self.id == 0 {
                            let n = <u64>::from_str_radix(
                                format!("{:?}", el.to_repr()).strip_prefix("0x").unwrap(),
                                16,
                            )
                            .unwrap();
                            println!("got {:?}", n);
                            println!("bytes {:?}", el.to_repr().0);
                        }
                        return;
                    }
                }
                // ignore
                _ => {}
            }
        }
    }
}
