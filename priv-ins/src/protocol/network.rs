use crate::protocol::NodeId;
use std::collections::HashMap;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Clone, Debug)]
pub enum Msg {}

pub type NetworkMessage = (NodeId, Msg);

#[async_trait::async_trait]
pub trait Network {
    fn send_to(&mut self, msg: NetworkMessage);
    async fn receive(&mut self) -> Option<NetworkMessage>;
    fn broadcast(&mut self, msg: Msg);
}

pub struct ChannelNetwork {
    id: NodeId,
    peers: HashMap<NodeId, UnboundedSender<NetworkMessage>>,
    receiver: UnboundedReceiver<NetworkMessage>,
}

impl ChannelNetwork {
    pub fn new(
        id: NodeId,
        peers: HashMap<NodeId, UnboundedSender<NetworkMessage>>,
        receiver: UnboundedReceiver<NetworkMessage>,
    ) -> Self {
        Self {
            id,
            peers,
            receiver,
        }
    }
}

#[async_trait::async_trait]
impl Network for ChannelNetwork {
    fn send_to(&mut self, msg: NetworkMessage) {
        if let Some(sender) = self.peers.get(&msg.0) {
            sender.send((self.id, msg.1)).expect("Should be open");
        }
    }

    async fn receive(&mut self) -> Option<NetworkMessage> {
        self.receiver.recv().await
    }

    fn broadcast(&mut self, msg: Msg) {
        for sender in self.peers.values() {
            sender.send((self.id, msg.clone())).expect("Should be open");
        }
    }
}
