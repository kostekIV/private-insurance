use std::collections::HashMap;
use std::ops::Deref;
use async_recursion::async_recursion;
use crate::crypto::shares::Share;
use crate::protocol::{DecoratedExpression, mul, mul_by_const, NodeId, Party, VarId};


pub struct Node<P> where P: Party {
    id: NodeId,
    party: P,
    variables: HashMap<VarId, u64>
}

impl<P> Node<P> where P: Party + Send {
    #[async_recursion]
    pub async fn evaluate(&mut self, expr: DecoratedExpression) -> Result<Share, String> {
        match expr {
            DecoratedExpression::AddConstant(s1, expression) => {
                if self.party.can_add(self.id) {
                    Ok(s1 + self.evaluate(*expression).await?)
                } else {
                    Ok(s1)
                }
            }
            DecoratedExpression::Add(left, right) => {
                Ok(self.evaluate(*left).await? + self.evaluate(*right).await?)
            }
            DecoratedExpression::Mul(a, b, id) => {
                let a = self.evaluate(*a).await?;
                let b = self.evaluate(*b).await?;
                Ok(mul(&id, a, b, &mut self.party).await)
            }
            DecoratedExpression::MulConstant(s1, expression) => {
                Ok(mul_by_const(&self.evaluate(*expression).await?, &s1))
            }
            DecoratedExpression::Var(id, var_id) => {
                if id == self.id {
                    let (r, rs) = self.party.open_self_input(id, var_id.clone()).await;
                    let v = self.variables.get(&var_id).ok_or("Node do not have its variable")?;
                    let vminusr = Share::from(*v) - r;

                    self.party.broadcast_self_input(id, var_id, vminusr).await;

                    Ok(vminusr + rs)
                } else {
                    Ok(self.party.get_input_shares(id, var_id).await)
                }
            }
            DecoratedExpression::Constant(share) => {
                Ok(share)
            }
        }
    }
}

