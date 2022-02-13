use num_traits::Num;
use serde::{Deserialize, Serialize};

type BExpression<T> = Box<Expression<T>>;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Expression<T: Num> {
    Number {
        number: T,
    },
    BinOp {
        left: BExpression<T>,
        right: BExpression<T>,
        op: BinaryOp,
    },
    Variable {
        name: String,
    },
}
