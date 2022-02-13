use crate::crypto::shares::{
    sum_elems, BeaverShare, Commitment, CommitmentProof, Elem, Share, Shares,
};
use crate::ff::PrimeField;
use std::collections::HashSet;
use std::{collections::HashMap, fmt::Debug, ops::Sub};

use crate::protocol::arithmetics::verify_commitments;
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
    proofs: HashMap<CirId, Vec<(NodeId, CommitmentProof)>>,
    commitments: HashMap<CirId, Vec<(NodeId, Commitment)>>,
    valid_proofs: HashSet<CirId>,
    invalid_proofs: HashSet<CirId>,
    original_shares: HashMap<CirId, Share>,
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
            proofs: HashMap::new(),
            valid_proofs: HashSet::new(),
            invalid_proofs: HashSet::new(),
            original_shares: HashMap::new(),
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

    fn handle_event(&mut self, event: NodeEvents, calculator: &Calculator) {
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
                    return;
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
                    return;
                }

                self.variable_salts.insert(c_id.clone(), s);
                self.combine_variable_if_full(c_id, &calculator);
            }
            NodeEvents::BeaverFor(c_id, beaver) => {
                if self.beavers.contains_key(&c_id) {
                    log::debug!("got twice value for {}", c_id);
                    return;
                }

                self.beavers.insert(c_id, beaver);
            }
            NodeEvents::NodeVariableShareReady(c_id, s) => {
                if self.variable_shares.contains_key(&c_id) {
                    log::debug!("got twice value for {}", c_id);
                    return;
                }

                self.variable_shares.insert(c_id.clone(), s);
                self.combine_variable_if_full(c_id, &calculator);
            }
            NodeEvents::CommitmentsFor(cir_id, commitments) => {
                if self.commitments.contains_key(&cir_id) {
                    log::debug!("got twice commitments for {}", cir_id);
                    return;
                }

                self.commitments.insert(cir_id, commitments);
            }
            NodeEvents::ProofsFor(cir_id, proofs) => {
                if self.proofs.contains_key(&cir_id) {
                    log::debug!("got twice proofs for {}", cir_id);
                    return;
                }

                self.proofs.insert(cir_id, proofs);
            }
            NodeEvents::ProofValid(cir_id) => {
                self.valid_proofs.insert(cir_id);
            }
            NodeEvents::ProofInvalid(cir_id) => {
                self.invalid_proofs.insert(cir_id);
            }
        }
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

        self.original_shares.insert(e_id.clone(), e.clone());
        self.original_shares.insert(f_id.clone(), f.clone());

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

        let e_x = calculator.generate_commitment_share(
            e_elem.clone(),
            self.original_shares.remove(&e_id).expect("checked"),
        );
        let f_x = calculator.generate_commitment_share(
            f_elem.clone(),
            self.original_shares.remove(&f_id).expect("checked"),
        );

        let (e_hash, e_salt) = Calculator::generate_commitment(&e_x);
        let (f_hash, f_salt) = Calculator::generate_commitment(&f_x);

        let e_proof = (e_hash, e_x.clone(), e_salt);
        let f_proof = (f_hash, f_x.clone(), f_salt);

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

    /// check state of node and make transition if needed
    fn state_transition(&self, state: NodeState) -> NodeState {
        match state {
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
        }
    }

    async fn wait_for_proofs(&mut self, calculator: &Calculator) {
        let mut need_proofs_for = self.my_proofs.keys().cloned().collect::<HashSet<_>>();

        for cir_id in self.proofs.keys() {
            need_proofs_for.remove(cir_id);
        }

        while !need_proofs_for.is_empty() {
            let event = match self.party_events.recv().await {
                Some(e) => e,
                None => {
                    log::debug!("party channel closed");
                    return;
                }
            };
            self.handle_event(event, &calculator);

            for cir_id in self.proofs.keys() {
                need_proofs_for.remove(cir_id);
            }
        }
    }

    fn check_proofs(&mut self) -> HashSet<CirId> {
        let cir_ids = self.proofs.keys().cloned().collect::<Vec<_>>();
        for cir_id in cir_ids.iter() {
            let mut proofs = self.proofs.remove(cir_id).expect("checked");

            let mut commits = self.commitments.remove(cir_id).expect("checked");

            proofs.sort_by(|a, b| a.0.cmp(&b.0));
            commits.sort_by(|a, b| a.0.cmp(&b.0));

            for ((a_id, proof), (b_id, comm)) in proofs.iter().zip(commits) {
                if *a_id != b_id {
                    panic!("this should be checked in party probably");
                }

                if proof.0 != comm {
                    self.party_commands
                        .send(NodeCommands::ProofInvalid(cir_id.clone()))
                        .expect("Send should succeed");
                    panic!("Abort");
                }
            }

            if !verify_commitments(&proofs.into_iter().map(|(_, c)| c).collect()) {
                self.party_commands
                    .send(NodeCommands::ProofInvalid(cir_id.clone()))
                    .expect("Send should succeed");
                panic!("Abort");
            }

            self.party_commands
                .send(NodeCommands::ProofVerified(cir_id.clone()))
                .expect("Send should succeed");
        }

        cir_ids.into_iter().collect()
    }

    async fn wait_for_others(&mut self, mut cir_ids: HashSet<CirId>, calculator: &Calculator) {
        while !cir_ids.is_empty() {
            if self.id == 0 {
                println!("xd {:?}", cir_ids);
            }
            for id in self.valid_proofs.iter() {
                cir_ids.remove(id);
            }
            if !self.invalid_proofs.is_empty() {
                panic!("Abort");
            }
            if cir_ids.is_empty() {
                return;
            }
            let event = match self.party_events.recv().await {
                Some(e) => e,
                None => {
                    log::debug!("party channel closed");
                    return;
                }
            };
            self.handle_event(event, &calculator);
        }
    }

    async fn try_proceed_with_mul(
        &mut self,
        state: NodeState,
        calculator: &Calculator,
    ) -> NodeState {
        match state {
            HaveBeaver(cir_id, ev1, ev2) => self.handle_beaver(&calculator, cir_id, ev1, ev2).await,
            HaveShares(cir_id, e_id, f_id, beaver) => {
                self.handle_shares(&calculator, cir_id, e_id, f_id, beaver)
            }
            s => s,
        }
    }

    pub async fn run(mut self, exp: DecoratedExpression) -> u64 {
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

            state = self.state_transition(state);

            if self.id == 0 {
                println!("NodeState: {:?}", state);
            }

            if self.can_proceed(&state) {
                let evaluating = circuit_nodes.get(idx).expect("we control it");
                state = self.try_proceed(&calculator, evaluating);
                idx += 1;
                continue;
            }

            state = self.try_proceed_with_mul(state, &calculator).await;

            // guard to avoid deadlock in situation when we are not sending or expecting any message
            // but can proceed without waiting for anything
            if self.can_proceed(&state) {
                idx += 1;
                continue;
            }

            let event = match self.party_events.recv().await {
                Some(e) => e,
                None => {
                    log::debug!("party channel closed");
                    panic!("abort");
                }
            };

            self.handle_event(event, &calculator);
        }

        // we have final share lets open it now
        let last_node_id = circuit_nodes
            .last()
            .expect("at least one should exist")
            .cir_id();

        // send all proofs
        for (cir_id, proof) in self.my_proofs.iter() {
            self.party_commands
                .send(NodeCommands::ProofFor(cir_id.clone(), proof.clone()))
                .expect("Send should succeed");
        }

        // wait for all proofs
        self.wait_for_proofs(&calculator).await;

        // check proofs
        let to_check = self.check_proofs();

        // wait for all nodes to conclude their checks
        self.wait_for_others(to_check, &calculator).await;

        self.evaluate_last(last_node_id, &calculator).await
    }

    async fn evaluate_last(&mut self, last_id: CirId, calculator: &Calculator) -> u64 {
        let evaluated = self
            .evaluated
            .remove(&last_id)
            .expect("we finished the evaluation");

        self.original_shares
            .insert(last_id.clone(), evaluated.clone());

        self.party_commands
            .send(NodeCommands::OpenShare(evaluated, last_id.clone()))
            .expect("should succeed");

        loop {
            let event = match self.party_events.recv().await {
                Some(e) => e,
                None => {
                    log::debug!("party channel closed");
                    panic!("abort");
                }
            };

            self.handle_event(event, &calculator);

            if self.fully_open.contains_key(&last_id) {
                break;
            }
        }

        let shares = self.fully_open.remove(&last_id).expect("checked");

        let ev_elem = sum_elems(&shares.into_iter().map(|(e, _)| e).collect());

        let n = <u64>::from_str_radix(
            format!("{:?}", ev_elem.to_repr())
                .strip_prefix("0x")
                .unwrap(),
            16,
        )
        .unwrap();
        let ev_x = calculator.generate_commitment_share(
            ev_elem.clone(),
            self.original_shares.remove(&last_id).expect("checked"),
        );

        let (ev_hash, ev_salt) = Calculator::generate_commitment(&ev_x);

        let ev_proof = (ev_hash, ev_x.clone(), ev_salt);

        self.my_proofs.clear();
        self.my_proofs.insert(last_id.clone(), ev_proof);

        self.party_commands
            .send(NodeCommands::CommitmentFor(last_id.clone(), ev_hash))
            .expect("send should succeed");

        loop {
            let event = match self.party_events.recv().await {
                Some(e) => e,
                None => {
                    log::debug!("party channel closed");
                    panic!("abort");
                }
            };

            self.handle_event(event, &calculator);

            if self.commitments.contains_key(&last_id) {
                break;
            }
        }

        for (cir_id, proof) in self.my_proofs.iter() {
            self.party_commands
                .send(NodeCommands::ProofFor(cir_id.clone(), proof.clone()))
                .expect("Send should succeed");
        }

        // wait for all proofs
        self.wait_for_proofs(&calculator).await;

        if self.id == 0 {
            println!("loop2 {:?}", last_id);
        }
        // check proofs
        self.check_proofs();

        // wait for all nodes to conclude their checks
        self.wait_for_others([last_id.clone()].into_iter().collect(), &calculator).await;

        // yay
        if self.id == 0 {
            println!("Got {:?}", n);
        }

        n
    }
}
