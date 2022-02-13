use crate::crypto::shares::{Commitment, CommitmentProof, Share, Shares};
use crate::protocol::{
    network::{Msg, Network},
    Alpha, CirId, DealerCommands, DealerEvents, NodeCommands, NodeEvents, NodeId,
};
use std::collections::{HashMap, HashSet};
use tokio::{
    select,
    sync::mpsc::{UnboundedReceiver as Receiver, UnboundedSender as Sender},
};

pub struct Party<N: Network + Send> {
    id: NodeId,
    dealer: (Sender<(NodeId, DealerCommands)>, Receiver<DealerEvents>),
    alpha_channel: Sender<Alpha>,
    node_commands: Receiver<NodeCommands>,
    node_events: Sender<NodeEvents>,
    network: N,
    shares_per: HashMap<CirId, Shares>,
    opened_shares: HashMap<NodeId, HashSet<CirId>>,
    commitments_per: HashMap<CirId, Vec<(NodeId, Commitment)>>,
    node_commitments: HashMap<NodeId, HashSet<CirId>>,
    proofs_per: HashMap<CirId, Vec<(NodeId, CommitmentProof)>>,
    node_proofs: HashMap<NodeId, HashSet<CirId>>,
    status_per: HashMap<CirId, u8>,
    node_statuses: HashMap<NodeId, HashSet<CirId>>,
    n_parties: u8,
}

impl<N: Network + Send> Party<N> {
    pub fn new(
        id: NodeId,
        dealer: (Sender<(NodeId, DealerCommands)>, Receiver<DealerEvents>),
        alpha_channel: Sender<Alpha>,
        node_commands: Receiver<NodeCommands>,
        node_events: Sender<NodeEvents>,
        network: N,
        n_parties: u8,
    ) -> Self {
        Self {
            id,
            dealer,
            alpha_channel,
            node_commands,
            node_events,
            network,
            n_parties,
            opened_shares: HashMap::new(),
            shares_per: HashMap::new(),
            commitments_per: HashMap::new(),
            node_commitments: HashMap::new(),
            proofs_per: HashMap::new(),
            node_proofs: HashMap::new(),
            status_per: HashMap::new(),
            node_statuses: HashMap::new(),
        }
    }
    /// collects share from given node for given circuit node.
    /// Checks for double sending
    /// If we have all shares returns true
    fn collect_share(&mut self, from: NodeId, share: Share, cid: CirId) -> bool {
        let opened_cirs = self.opened_shares.entry(from).or_insert(HashSet::new());

        if !opened_cirs.insert(cid.clone()) {
            log::debug!(
                "node {} tried to open more than once its share for circuit node {}",
                from,
                cid.clone()
            );

            // Return notready to not trigger twice ready logic.
            return false;
        }

        let shares = self.shares_per.entry(cid).or_insert(Shares::new());
        shares.push(share);

        shares.len() == self.n_parties as usize
    }

    /// similar to collect_share but collects commitments
    fn collect_commitment(&mut self, from: NodeId, comm: Commitment, cid: CirId) -> bool {
        let commited_to = self.node_commitments.entry(from).or_insert(HashSet::new());

        if !commited_to.insert(cid.clone()) {
            log::debug!(
                "node {} tried to submit more than once its commitment for circuit node {}",
                from,
                cid.clone()
            );

            // Return notready to not trigger twice ready logic.
            return false;
        }

        let comms = self.commitments_per.entry(cid).or_insert(Vec::new());
        comms.push((from, comm));

        comms.len() == self.n_parties as usize
    }

    fn collect_proof(&mut self, from: NodeId, proof: CommitmentProof, cid: CirId) -> bool {
        let proofs = self.node_proofs.entry(from).or_insert(HashSet::new());

        if !proofs.insert(cid.clone()) {
            log::debug!(
                "node {} tried to submit more than once its proof for circuit node {}",
                from,
                cid.clone()
            );

            // Return notready to not trigger twice ready logic.
            return false;
        }

        let ps = self.proofs_per.entry(cid).or_insert(Vec::new());
        ps.push((from, proof));

        ps.len() == self.n_parties as usize
    }

    fn collect_status(&mut self, from: NodeId, valid: bool, cid: CirId) -> bool {
        let statuses = self.node_statuses.entry(from).or_insert(HashSet::new());

        if !statuses.insert(cid.clone()) {
            log::debug!(
                "node {} tried to submit more than once its status for circuit node {}",
                from,
                cid.clone()
            );

            // Return notready to not trigger twice ready logic.
            return false;
        }

        if !valid {
            return true;
        }

        let count = self.status_per.entry(cid).or_insert(0);
        *count += 1;

        *count == self.n_parties
    }

    fn handle_network_msg(&mut self, from: NodeId, msg: Msg) {
        if self.id == 0 {
            println!("NetworkMsg from {:?} {:?}", from, msg);
        }
        match msg {
            Msg::OpenShare(cid, share) => {
                if self.collect_share(from, share, cid.clone()) {
                    let collected_shares =
                        self.shares_per.remove(&cid).expect("We have collected it");
                    self.node_events
                        .send(NodeEvents::CirReady(cid, collected_shares))
                        .expect("Send should succeed");
                }
            }
            Msg::OpenVariable(cid, elem) => {
                self.node_events
                    .send(NodeEvents::NodeVariableReady(cid, elem))
                    .expect("Send should succeed");
            }
            Msg::Commit(cid, comm) => {
                if self.collect_commitment(from, comm, cid.clone()) {
                    let comms = self
                        .commitments_per
                        .remove(&cid)
                        .expect("We have collected it");
                    self.node_events
                        .send(NodeEvents::CommitmentsFor(cid, comms))
                        .expect("Send should succeed");
                }
            }
            Msg::Proof(cid, proof) => {
                if self.collect_proof(from, proof, cid.clone()) {
                    let proofs = self.proofs_per.remove(&cid).expect("We have collected it");
                    self.node_events
                        .send(NodeEvents::ProofsFor(cid, proofs))
                        .expect("Send should succeed");
                }
            }
            Msg::ProofInvalid(cid) => {
                if self.collect_status(from, false, cid.clone()) {
                    self.node_events
                        .send(NodeEvents::ProofInvalid(cid))
                        .expect("Send should succeed");
                }
            }
            Msg::ProofValid(cid) => {
                if self.collect_status(from, true, cid.clone()) {
                    self.node_events
                        .send(NodeEvents::ProofValid(cid))
                        .expect("Send should succeed");
                }
            }
        }
    }

    fn handle_node_command(&mut self, cmd: NodeCommands) {
        if self.id == 0 {
            println!("NodeCmd {:?}", cmd);
        }

        match cmd {
            NodeCommands::OpenShare(share, cir_id) => {
                self.network.broadcast(Msg::OpenShare(cir_id, share));
            }
            NodeCommands::OpenSelfInput(v_id) => {
                self.dealer
                    .0
                    .send((self.id, DealerCommands::NodeOpenSelfInput(v_id)))
                    .expect("Send should succeed");
            }
            NodeCommands::NeedBeaver(cir_id) => {
                self.dealer
                    .0
                    .send((self.id, DealerCommands::BeaverFor(cir_id)))
                    .expect("Send should succeed");
            }
            NodeCommands::OpenSelfShare(s, cir_id) => {
                self.network.broadcast(Msg::OpenVariable(cir_id, s))
            }
            NodeCommands::NeedAlpha => {
                self.dealer
                    .0
                    .send((self.id, DealerCommands::NeedAlpha))
                    .expect("Send should succeed");
            }
            NodeCommands::CommitmentFor(cir_id, comm) => {
                self.network.broadcast(Msg::Commit(cir_id, comm));
            }
            NodeCommands::ProofFor(cir_id, proof) => {
                self.network.broadcast(Msg::Proof(cir_id, proof));
            }
            NodeCommands::ProofVerified(cir_id) => {
                self.network.broadcast(Msg::ProofValid(cir_id));
            }
            NodeCommands::ProofInvalid(cir_id) => {
                self.network.broadcast(Msg::ProofInvalid(cir_id));
            }
        }
    }

    fn handle_dealer_event(&mut self, dealer_event: DealerEvents) {
        if self.id == 0 {
            println!("DealerEvent::{:?}", dealer_event);
        }

        match dealer_event {
            DealerEvents::NodeSelfVariable(var_id, r, r_share) => {
                self.node_events
                    .send(NodeEvents::SelfVariableReady(var_id, r, r_share))
                    .expect("Send should succeed");
            }
            DealerEvents::NodeVariableShared(var_id, r_share) => {
                self.node_events
                    .send(NodeEvents::NodeVariableShareReady(var_id, r_share))
                    .expect("Send should succeed");
            }
            DealerEvents::BeaverSharesFor(cir_id, beaver_shares) => {
                self.node_events
                    .send(NodeEvents::BeaverFor(cir_id, beaver_shares))
                    .expect("Send should succeed");
            }
            DealerEvents::Alpha(alpha) => {
                self.alpha_channel
                    .send(Alpha(alpha))
                    .expect("Send should succeed");
            }
        }
    }

    pub async fn run(&mut self) {
        loop {
            select! {
                Some((from, msg)) = self.network.receive() => self.handle_network_msg(from, msg),
                node_command = self.node_commands.recv() => {
                    let cmd = match node_command {
                        None => {
                            log::debug!("Cmd channel closed");
                            continue;
                        },
                        Some(cmd) => cmd,
                    };
                    self.handle_node_command(cmd);
                },
                dealer_msg = self.dealer.1.recv() => {
                    let dealer_event = match dealer_msg {
                        Some(e) => e,
                        None => {
                            log::debug!("dealer channel closed!");
                            continue;
                        }
                    };

                    self.handle_dealer_event(dealer_event);
                }
            }
        }
    }
}
