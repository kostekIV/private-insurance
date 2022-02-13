use crate::expressions::BinaryOp::{Add, Mul};
use crate::expressions::Expression;
use crate::protocol::run_nodes;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct SuccessMsg {
    msg: String,
}

#[tokio::test]
async fn test_expression_small() {
    let expression = Expression::<u64>::BinOp {
        left: Box::new(Expression::Variable {
            name: "0".to_string(),
        }),
        right: Box::new(Expression::Variable {
            name: "1".to_string(),
        }),
        op: Mul,
    };
    let variables = (0..2)
        .map(|i| [(i.to_string(), i + 5)].iter().cloned().collect())
        .collect();
    let expected_result = 5 * 6;
    let results = run_nodes(2, variables, expression).await;
    assert_eq!(
        results.into_iter().map(|r| r.unwrap()).collect::<Vec<_>>(),
        (0..2).map(|_| expected_result).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn test_more_parties_then_variables() {
    let expression = Expression::<u64>::BinOp {
        left: Box::new(Expression::Variable {
            name: "0".to_string(),
        }),
        right: Box::new(Expression::Variable {
            name: "1".to_string(),
        }),
        op: Mul,
    };
    let variables = (0..4)
        .map(|i| [(i.to_string(), i + 5)].iter().cloned().collect())
        .collect();
    let expected_result = 5 * 6;
    let results = run_nodes(4, variables, expression).await;
    assert_eq!(
        results.into_iter().map(|r| r.unwrap()).collect::<Vec<_>>(),
        (0..4).map(|_| expected_result).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn test_expression_big() {
    let expression = Expression::<u64>::BinOp {
        left: Box::new(Expression::<u64>::BinOp {
            left: Box::new(Expression::<u64>::BinOp {
                left: Box::new(Expression::<u64>::BinOp {
                    left: Box::new(Expression::<u64>::BinOp {
                        left: Box::new(Expression::Number { number: 10 }),
                        right: Box::new(Expression::Variable {
                            name: "0".to_string(),
                        }),
                        op: Mul,
                    }),
                    right: Box::new(Expression::Variable {
                        name: "1".to_string(),
                    }),
                    op: Mul,
                }),
                right: Box::new(Expression::Variable {
                    name: "2".to_string(),
                }),
                op: Mul,
            }),
            right: Box::new(Expression::Variable {
                name: "3".to_string(),
            }),
            op: Mul,
        }),
        right: Box::new(Expression::Variable {
            name: "4".to_string(),
        }),
        op: Add,
    };
    let variables = (0..5)
        .map(|i| [(i.to_string(), i + 5)].iter().cloned().collect())
        .collect();
    let expected_result = 10 * 5 * 6 * 7 * 8 + 9;
    let results = run_nodes(5, variables, expression).await;
    assert_eq!(
        results.into_iter().map(|r| r.unwrap()).collect::<Vec<_>>(),
        (0..5).map(|_| expected_result).collect::<Vec<_>>()
    );
}
