use crate::crypto::shares::Elem;
use crate::expressions::{BinaryOp, Expression};
use crate::protocol::{CirId, NodeId, Provider, VarId};

type BExpression = Box<DecoratedExpression>;

/// Enum representing preprocessed raw expression.
/// Each node gets unique id.
pub enum DecoratedExpression {
    /// addition of constant to nonconst expression
    AddConstant(Elem, BExpression, CirId),
    /// addition of two nonconst expressions
    Add(BExpression, BExpression, CirId),
    /// multiplication of two nonconst expression
    Mul(BExpression, BExpression, CirId),
    /// multiplication of constant to nonconst expression
    MulConstant(Elem, BExpression, CirId),
    /// node representing variable belonging to node
    Var(NodeId, VarId, CirId),
    /// constant node, only used during preprocessing phase(unless whole raw expression was constant)
    /// or constant equivalent (without any variables).
    Constant(Elem, CirId),
}

/// Expression repr after processing, all non const expressions are removed and substituted for ids
#[derive(Debug)]
pub enum MidEvalExpression {
    AddConstant(Elem, CirId, CirId),
    Add(CirId, CirId, CirId),
    MulConstant(Elem, CirId, CirId),
    Mul(CirId, CirId, CirId),
    Var(CirId),
}

impl MidEvalExpression {
    /// returns circuit node id
    pub fn cir_id(&self) -> CirId {
        match self {
            MidEvalExpression::AddConstant(_, _, id) => id.clone(),
            MidEvalExpression::Add(_, _, id) => id.clone(),
            MidEvalExpression::Mul(_, _, id) => id.clone(),
            MidEvalExpression::MulConstant(_, _, id) => id.clone(),
            MidEvalExpression::Var(id) => id.clone(),
        }
    }
}

impl DecoratedExpression {
    /// returns circuit node id
    pub fn cir_id(&self) -> CirId {
        match self {
            DecoratedExpression::AddConstant(_, _, id) => id.clone(),
            DecoratedExpression::Add(_, _, id) => id.clone(),
            DecoratedExpression::Mul(_, _, id) => id.clone(),
            DecoratedExpression::MulConstant(_, _, id) => id.clone(),
            DecoratedExpression::Var(_, _, id) => id.clone(),
            DecoratedExpression::Constant(_, id) => id.clone(),
        }
    }

    /// returns all ids of multiplication in expression
    pub fn mul_ids(&self) -> Vec<CirId> {
        match self {
            DecoratedExpression::AddConstant(_, e, _) => e.mul_ids(),
            DecoratedExpression::Add(e1, e2, _) => {
                let mut x = e1.mul_ids();
                x.extend(e2.mul_ids());
                x
            }
            DecoratedExpression::Mul(e1, e2, cir_id) => {
                let mut x = e1.mul_ids();
                x.extend(e2.mul_ids());
                x.push(cir_id.clone());
                x
            }
            DecoratedExpression::MulConstant(_, e, _) => e.mul_ids(),
            _ => vec![],
        }
    }

    /// returns all ids of variables that belong to node (or all variables if node is none)
    pub fn self_var_ids(&self, node_id: Option<NodeId>) -> Vec<(CirId, VarId)> {
        match self {
            DecoratedExpression::AddConstant(_, e, _) => e.self_var_ids(node_id),
            DecoratedExpression::Add(e1, e2, _) => {
                let mut x = e1.self_var_ids(node_id);
                x.extend(e2.self_var_ids(node_id));
                x
            }
            DecoratedExpression::Mul(e1, e2, _) => {
                let mut x = e1.self_var_ids(node_id);
                x.extend(e2.self_var_ids(node_id));
                x
            }
            DecoratedExpression::MulConstant(_, e, _) => e.self_var_ids(node_id),
            DecoratedExpression::Var(other, var_id, cir_id) => {
                if node_id.is_none() || node_id.expect("not none") == *other {
                    vec![(cir_id.clone(), var_id.clone())]
                } else {
                    vec![]
                }
            }
            DecoratedExpression::Constant(_, _) => {
                vec![]
            }
        }
    }

    /// transform expression to vector of `MidEvalExpression`.
    /// the order is safe for evaluating given expression
    pub fn into_ordered(self) -> Vec<MidEvalExpression> {
        match self {
            DecoratedExpression::AddConstant(s, e, cir_id) => {
                let e_cir_id = e.cir_id();

                let mut ord = e.into_ordered();
                ord.push(MidEvalExpression::AddConstant(s, e_cir_id, cir_id));

                ord
            }
            DecoratedExpression::Add(e1, e2, cir_id) => {
                let e1_id = e1.cir_id();
                let e2_id = e2.cir_id();

                let mut ord = e1.into_ordered();
                ord.extend(e2.into_ordered());
                ord.push(MidEvalExpression::Add(e1_id, e2_id, cir_id));

                ord
            }
            DecoratedExpression::Mul(e1, e2, cir_id) => {
                let e1_id = e1.cir_id();
                let e2_id = e2.cir_id();

                let mut ord = e1.into_ordered();
                ord.extend(e2.into_ordered());
                ord.push(MidEvalExpression::Mul(e1_id, e2_id, cir_id));

                ord
            }
            DecoratedExpression::MulConstant(s, e, cir_id) => {
                let e_cir_id = e.cir_id();

                let mut ord = e.into_ordered();
                ord.push(MidEvalExpression::MulConstant(s, e_cir_id, cir_id));

                ord
            }
            DecoratedExpression::Var(_, _, cir_id) => {
                vec![MidEvalExpression::Var(cir_id)]
            }
            DecoratedExpression::Constant(_, _) => {
                /// In the decorated expression all constants are only temporary
                vec![]
            }
        }
    }
}

/// Transform raw expression into
pub fn decorate_expression(
    expr: Expression<u64>,
    id_provider: &mut Provider,
) -> Result<DecoratedExpression, String> {
    match expr {
        Expression::Number { number } => Ok(DecoratedExpression::Constant(
            Elem::from(number),
            id_provider.next(),
        )),
        Expression::BinOp { left, right, op } => {
            let left = decorate_expression(*left, id_provider)?;
            let right = decorate_expression(*right, id_provider)?;

            match op {
                BinaryOp::Add => match (left, right) {
                    (
                        DecoratedExpression::Constant(s1, _),
                        DecoratedExpression::Constant(s2, _),
                    ) => Ok(DecoratedExpression::Constant(s1 + s2, id_provider.next())),
                    (DecoratedExpression::Constant(s1, _), x) => Ok(
                        DecoratedExpression::AddConstant(s1, Box::new(x), id_provider.next()),
                    ),
                    (x, DecoratedExpression::Constant(s1, _)) => Ok(
                        DecoratedExpression::AddConstant(s1, Box::new(x), id_provider.next()),
                    ),
                    (x, y) => Ok(DecoratedExpression::Add(
                        Box::new(x),
                        Box::new(y),
                        id_provider.next(),
                    )),
                },
                BinaryOp::Mul => match (left, right) {
                    (
                        DecoratedExpression::Constant(s1, _),
                        DecoratedExpression::Constant(s2, _),
                    ) => Ok(DecoratedExpression::Constant(s1 + s2, id_provider.next())),
                    (DecoratedExpression::Constant(s1, _), x) => Ok(
                        DecoratedExpression::MulConstant(s1, Box::new(x), id_provider.next()),
                    ),
                    (x, DecoratedExpression::Constant(s1, _)) => Ok(
                        DecoratedExpression::MulConstant(s1, Box::new(x), id_provider.next()),
                    ),
                    (x, y) => {
                        let id = id_provider.next();
                        Ok(DecoratedExpression::Mul(Box::new(x), Box::new(y), id))
                    }
                },
                _ => Err(format!("Only add and mul for now")),
            }
        }
        Expression::Variable { name } => {
            let node_id = id_provider
                .var_to_node(name.clone())
                .ok_or(format!("orphaned variable"))?;
            Ok(DecoratedExpression::Var(node_id, name, id_provider.next()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ff::Field;

    fn dummy() -> Elem {
        Elem::zero()
    }

    fn test_expr() -> DecoratedExpression {
        DecoratedExpression::Add(
            Box::new(DecoratedExpression::MulConstant(
                dummy(),
                Box::new(DecoratedExpression::Mul(
                    Box::new(DecoratedExpression::Var(
                        1,
                        "2".to_string(),
                        "1".to_string(),
                    )),
                    Box::new(DecoratedExpression::Var(
                        1,
                        "3".to_string(),
                        "2".to_string(),
                    )),
                    "3".to_string(),
                )),
                "4".to_string(),
            )),
            Box::new(DecoratedExpression::Mul(
                Box::new(DecoratedExpression::Var(
                    3,
                    "5".to_string(),
                    "5".to_string(),
                )),
                Box::new(DecoratedExpression::Var(
                    1,
                    "10".to_string(),
                    "6".to_string(),
                )),
                "7".to_string(),
            )),
            "8".to_string(),
        )
    }

    #[test]
    fn returns_mul_ids_correctly() {
        let d_expr = test_expr();

        assert_eq!(vec!["3", "7"], d_expr.mul_ids());
    }

    #[test]
    fn returns_var_ids_correctly() {
        let d_expr = test_expr();

        assert_eq!(
            vec![
                ("1".to_string(), "2".to_string()),
                ("2".to_string(), "3".to_string()),
                ("6".to_string(), "10".to_string())
            ],
            d_expr.self_var_ids(Some(1))
        );
        assert_eq!(Vec::<(CirId, VarId)>::new(), d_expr.self_var_ids(Some(2)));
        assert_eq!(
            vec![("5".to_string(), "5".to_string())],
            d_expr.self_var_ids(Some(3))
        );
    }

    #[test]
    fn produce_correct_order() {
        let d_expr = test_expr();

        let ordered_ids = d_expr
            .into_ordered()
            .iter()
            .map(|e| e.cir_id())
            .collect::<Vec<_>>();

        assert_eq!(vec!["1", "2", "3", "4", "5", "6", "7", "8"], ordered_ids);
    }
}
