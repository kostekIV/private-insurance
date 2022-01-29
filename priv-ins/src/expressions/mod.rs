use serde::{Deserialize, Serialize};

type BExpression<T> = Box<Expression<T>>;

#[derive(Deserialize, Serialize, Debug)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Expression<T> {
    Number {
        number: T,
    },
    BinOp {
        left: BExpression<T>,
        right: BExpression<T>,
        op: BinaryOp,
    },
}

#[cfg(test)]
mod tests {
    use crate::expressions::BinaryOp::Add;
    use crate::expressions::Expression;

    #[test]
    fn it_works() {
        let x = Expression::BinOp {
            left: Box::new(Expression::Number { number: 10.2 }),
            right: Box::new(Expression::Number { number: 10.2 }),
            op: Add,
        };

        let serialized = serde_json::to_string(&x).unwrap();
        println!("serialized = {}", serialized);
    }
}
