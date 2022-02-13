#[cfg(test)]
mod test {
    use crate::protocol::network::setup_network;

    #[tokio::test]
    async fn test() {
        let networks = setup_network(4);
    }
}
