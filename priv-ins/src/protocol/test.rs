use crate::protocol::{network::ChannelNetwork, NodeId};
use std::collections::HashMap;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

fn setup_network(n: u32) -> Vec<ChannelNetwork> {
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

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test() {
        let networks = setup_network(4);
    }
}
