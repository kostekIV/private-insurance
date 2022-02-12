use crate::crypto::shares::{BeaverShare, Share};
use crate::protocol::{CirId, NodeId, VarId};

use tokio::sync::channel;
use tokio::sync::mpsc::{Receiver, Sender};

enum DealerMessage {

}

struct Party<N: Network> {
    dealer: (Sender<DealerMessage>, Receiver<DealerMessage>),
    network: N
}

#[async_trait::async_trait]
impl<N: Network> crate::protocol::Party for Party<N> {
    async fn open(&mut self, id: &CirId, value: Share) -> Share {
        todo!()
    }

    async fn beaver_for(&mut self, id: &CirId) -> BeaverShare {
        todo!()
    }

    async fn open_self_input(&mut self, nid: NodeId, vid: VarId) -> (Share, Share) {
        todo!()
    }

    async fn broadcast_self_input(&mut self, nid: NodeId, vid: VarId, share: Share) {
        todo!()
    }

    async fn get_input_shares(&mut self, nid: NodeId, vid: VarId) -> Share {
        todo!()
    }

    /// Allow only first node to add constants
    fn can_add(&self, nid: NodeId) -> bool {
        nid == 0
    }
}