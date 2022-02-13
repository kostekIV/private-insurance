use num_traits::Float;
use std::collections::HashMap;
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
pub enum Expression<T: Float> {
    Number {
        number: T,
    },
    BinOp {
        left: BExpression<T>,
        right: BExpression<T>,
        op: BinaryOp,
    },
    Variable {
        name: String
    },
}

pub fn eval_expression<T: Float>(exp: &Expression<T>, var_mapping: &HashMap<String, T>) -> Result<T, String> {
    match exp {
        Expression::Number { number } => { Ok(*number) }
        Expression::BinOp { left, right, op } => {
            Ok(match op {
                BinaryOp::Add => {
                    eval_expression(left, var_mapping)? + eval_expression(right, var_mapping)?
                }
                BinaryOp::Sub => {
                    eval_expression(left, var_mapping)? - eval_expression(right, var_mapping)?
                }
                BinaryOp::Mul => {
                    eval_expression(left, var_mapping)? * eval_expression(right, var_mapping)?
                }
                BinaryOp::Div => {
                    eval_expression(left, var_mapping)? / eval_expression(right, var_mapping)?
                }
            })
        }
        Expression::Variable { name } => {
            var_mapping
                .get(name)
                .ok_or(format!("Variable `{}` not found", name))
                .map(|&x| x)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use crate::expressions::BinaryOp::Add;
    use crate::expressions::{eval_expression, Expression};

    #[test]
    fn it_works() {
        let x = Expression::<f32>::BinOp {
            left: Box::new(Expression::Number { number: 10.2 }),
            right: Box::new(Expression::Variable { name: "x".to_string()}),
            op: Add,
        };

        assert_eq!(Ok(20.2), eval_expression(&x, &HashMap::from([(String::from("x"), 10.0)])));
        assert_eq!(Ok(21.2), eval_expression(&x, &HashMap::from([(String::from("x"), 11.0)])));
        assert_eq!(Ok(22.2), eval_expression(&x, &HashMap::from([(String::from("x"), 12.0)])));
        assert_eq!(Err("Variable `x` not found".to_string()), eval_expression(&x, &HashMap::from([])));
    }
}
