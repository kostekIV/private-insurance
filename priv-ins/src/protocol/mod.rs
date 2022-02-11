mod dealer;

#[async_trait::async_trait]
pub trait Party {
    /// returns r, [r] for NodeID
    async fn open_self_input(&mut self, id: NodeId) -> (Share, Share);
    /// returns share for NodeId [r]
    async fn get_input_shares(&mut self, id: NodeId) -> Share;
}