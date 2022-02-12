use crate::protocol::NodeId;

pub enum Msg {

}
pub type NetworkMessage = (NodeId, Msg);

#[async_trait::async_trait]
trait Network {
    async fn send_to(msg: NetworkMessage);
    async fn receive() -> NetworkMessage;
    async fn broadcast();
}



