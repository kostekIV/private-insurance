use crate::crypto::shares::{Elem, Share};
use crate::protocol::{CirId, NodeId};
use std::collections::HashMap;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

#[derive(Clone, Debug)]
pub enum Msg {
    OpenShare(CirId, Share),
    OpenVariable(CirId, Elem),
}

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

pub fn setup_network(n: u32) -> Vec<ChannelNetwork> {
    let (senders, receivers): (Vec<_>, Vec<_>) = (0..n).map(|_| unbounded_channel()).unzip();
    receivers
        .into_iter()
        .enumerate()
        .map(|(id, receiver)| {
            let peers: HashMap<_, _> = senders
                .iter()
                .enumerate()
                .map(|(i, s)| ((i + 1) as NodeId, s.clone()))
                .collect();
            ChannelNetwork::new((id + 1) as NodeId, peers, receiver)
        })
        .collect()
}

#[async_trait::async_trait]
impl Network for ChannelNetwork {
    fn send_to(&mut self, msg: NetworkMessage) {
        if self.id == 0 {
            println!("Network::send_to {:?}", msg.0);
        }
        if let Some(sender) = self.peers.get(&msg.0) {
            sender.send((self.id, msg.1)).expect("Should be open");
        }
    }

    async fn receive(&mut self) -> Option<NetworkMessage> {
        self.receiver.recv().await
    }
    fn broadcast(&mut self, msg: Msg) {
        if self.id == 0 {
            println!("Network::broadcast");
        }
        for sender in self.peers.values() {
            sender.send((self.id, msg.clone())).expect("Should be open");
        }
    }
}
