use std::collections::{HashMap, HashSet};
use tokio::select;
use crate::crypto::shares::{BeaverShare, Share, Shares};
use crate::protocol::{CirId, NodeId, VarId};
use crate::protocol::network::{Msg, Network};

use tokio::sync::mpsc::{UnboundedReceiver as Receiver, UnboundedSender as Sender};
use futures::prelude::*;

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

struct Party<N: Network + Send> {
    dealer: (Sender<DealerCommands>, Receiver<DealerEvents>),
    node_commands: Receiver<NodeCommands>,
    node_events: Sender<NodeEvents>,
    network: N,
    shares_per: HashMap<CirId, Shares>,
    opened_shares: HashMap<NodeId, HashSet<CirId>>,
    n_parties: u8,
}

enum IsReady {
    NotReady,
    Ready,
}

impl<N: Network + Send> Party<N> {
    async fn gunwo(mut self) {
        let x = match self.node_commands.recv().await {
            None => { return (); }
            Some(x) => { x }
        };
    }

    fn collect_share(&mut self, from: NodeId, share: Share, cid: CirId) -> bool {
        let opened_cirs = self.opened_shares.entry(from).or_insert(HashSet::new());

        if !opened_cirs.insert(cid.clone()) {
            log::debug!("node {} tried to open more than once its share for circuit node {}", from, cid.clone());

            // Return notready to not trigger twice ready logic.
            return false;
        }

        let shares = self.shares_per.entry(cid).or_insert(Shares::new());
        shares.push(share);

        shares.len() == self.n_parties as usize
    }

    async fn run(&mut self) {
        loop {
            select! {
                Some((from, msg)) = self.network.receive() => {
                    match msg {
                        Msg::OpenShare(cid, share) => {
                            if self.collect_share(from, share, cid.clone()) {
                                let collected_shares = self.shares_per.remove(&cid).expect("We have collected it");
                                self.node_events.send(NodeEvents::CirReady(cid, collected_shares)).expect("Send should succeed");
                            }
                        }
                    }
                },
                node_command = self.node_commands.recv() => {
                    let cmd = match node_command {
                        None => {
                            log::debug!("Cmd channel closed");
                            continue;
                        },
                        Some(cmd) => cmd,
                    };

                    match cmd {
                        NodeCommands::OpenShare(share, cir_id) => {
                            self.network.broadcast(Msg::OpenShare(cir_id, share));
                        },
                        NodeCommands::OpenSelfInput(v_id) => {
                            self.dealer.0.send(
                                DealerCommands::NodeOpenSelfInput(v_id)
                            ).expect("Send should succeed");
                        },
                        NodeCommands::NeedBeaver(cir_id) => {
                            self.dealer.0.send(
                                DealerCommands::BeaverFor(cir_id)
                            ).expect("Send should succeed");
                        }
                    }
                },
                dealer_msg = self.dealer.1.recv() => {
                    let dealer_event = match dealer_msg {
                        Some(e) => e,
                        None => {
                            log::debug!("dealer channel closed!");
                            continue;
                        }
                    };

                    match dealer_event {
                        DealerEvents::NodeSelfVariable(var_id, r, r_share) => {
                            self.node_events.send(
                                NodeEvents::SelfVariableReady(var_id, r, r_share)
                            ).expect("Send should succeed");
                        }
                        DealerEvents::NodeVariableShared(var_id, r_share) => {
                            self.node_events.send(
                                NodeEvents::NodeVariableReady(var_id, r_share)
                            ).expect("Send should succeed");
                        }
                        DealerEvents::BeaverSharesFor(cir_id, beaver_shares) => {
                            self.node_events.send(
                                NodeEvents::BeaverFor(cir_id, beaver_shares)
                            ).expect("Send should succeed");
                        }
                    }
                }
            }
        }
    }
}
