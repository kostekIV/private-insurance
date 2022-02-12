use crate::crypto::shares::Share;
use crate::protocol::{CirId, NodeId};

pub enum Msg {
    OpenShare(CirId, Share)
}

pub type NetworkMessage = (NodeId, Msg);

#[async_trait::async_trait]
pub trait Network {
    async fn send_to(&mut self, msg: NetworkMessage);
    async fn receive(&mut self) -> NetworkMessage;
    async fn broadcast(&mut self, msg: Msg);
}



