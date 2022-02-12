use std::collections::HashMap;
use crate::crypto::shares::{Beaver, Shares};
use crate::protocol::{CirId, Dealer, NodeId, VarId};

struct TrustedDealer {
    n_parties: u8,
    beavers: HashMap<CirId, Beaver>,
    variables: HashMap<VarId, Shares>,
    variables_ownership: HashMap<VarId, NodeId>
}

#[async_trait::async_trait]
impl Dealer for TrustedDealer {
    async fn new_beaver(&mut self, id: &CirId) {
        let b = crate::crypto::shares::random_beaver(self.n_parties);

        self.beavers.insert(id.clone(), b);
    }

    async fn prepare_variable(&mut self, nid: &NodeId, vid: &VarId) {
        let shares = crate::crypto::shares::random_shares(self.n_parties);

        self.variables.insert(vid.to_string(), shares);
        self.variables_ownership.insert(vid.to_string(), nid.clone());
    }
}

struct RemoteDealer {

}