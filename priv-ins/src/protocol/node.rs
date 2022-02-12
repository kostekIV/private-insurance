use std::collections::HashMap;
use std::ops::Deref;
use async_recursion::async_recursion;
use crate::crypto::shares::Share;
use crate::protocol::{DecoratedExpression, NodeCommands, NodeEvents, NodeId, VarId};

use tokio::sync::mpsc::{UnboundedReceiver as Receiver, UnboundedSender as Sender};
use futures::prelude::*;


pub struct Node {
    id: NodeId,
    party_commands: Sender<NodeCommands>,
    party_events: Sender<NodeEvents>,
    variables: HashMap<VarId, u64>,
}

impl Node {
    pub async fn run(mut self) {

    }
}

