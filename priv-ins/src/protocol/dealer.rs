use crate::crypto::{
    shares::{self, BeaverShare, Elem, Shares},
    Fp,
};
use crate::protocol::{CirId, DealerCommands, DealerEvents, NodeId, VarId};
use ff::Field;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

struct TrustedDealer {
    n_parties: u8,
    alpha: (Elem, Vec<Elem>),
    beavers: HashMap<CirId, Vec<BeaverShare>>,
    variables_ownership: HashMap<VarId, NodeId>,
    senders: HashMap<NodeId, UnboundedSender<DealerEvents>>,
    receiver: UnboundedReceiver<(NodeId, DealerCommands)>,
}

impl TrustedDealer {
    async fn run(mut self) {
        loop {
            match self.receiver.recv().await {
                Some((node_id, DealerCommands::BeaverFor(cir_id))) => {
                    let shares = match self.beavers.entry(cir_id.clone()) {
                        Entry::Occupied(o) => o.into_mut(),
                        Entry::Vacant(v) => {
                            v.insert(shares::random_beaver(&self.alpha.1, self.n_parties))
                        }
                    };

                    if let Some(sender) = self.senders.get(&node_id) {
                        sender
                            .send(DealerEvents::BeaverSharesFor(
                                cir_id,
                                *shares
                                    .get(node_id as usize)
                                    .expect("Dealer shoud have share"),
                            ))
                            .expect("Dealer shoud have sender");
                    }
                }
                Some((node_id, DealerCommands::NeedAlpha)) => {
                    if let Some(sender) = self.senders.get(&node_id) {
                        sender
                            .send(DealerEvents::Alpha(self.alpha.1[node_id as usize]))
                            .expect("Dealer shoud have sender");
                    }
                }
                Some((node_id, DealerCommands::NodeOpenSelfInput(cir_id))) => {
                    if !self.variables_ownership.contains_key(&cir_id) {
                        self.variables_ownership
                            .insert(cir_id.clone(), node_id)
                            .expect("Dealer should be able to insert key");
                        let r = Elem::random(rand::thread_rng());
                        let shares = shares::shares_from_secret(&r, &self.alpha.1, self.n_parties);
                        for (i, share) in shares.iter().enumerate() {
                            if i != (node_id as usize) {
                                if let Some(sender) = self.senders.get(&(i as u64)) {
                                    sender
                                        .send(DealerEvents::NodeVariableShared(
                                            cir_id.clone(),
                                            *share,
                                        ))
                                        .expect("Dealer shoud have sender");
                                }
                            }
                        }
                        if let Some(sender) = self.senders.get(&node_id) {
                            sender
                                .send(DealerEvents::NodeSelfVariable(
                                    cir_id.clone(),
                                    r,
                                    shares[node_id as usize],
                                ))
                                .expect("Dealer shoud have sender");
                        }
                    }
                }
                None => {}
            };
        }
    }
}
