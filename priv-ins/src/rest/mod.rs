use std::collections::HashMap;
use crate::expressions::{eval_expression, Expression, BinaryOp};
use tide::{Body, Request};

use serde::{Deserialize, Serialize};
use tide::log::{log, Level};

use num_traits::Float;

#[derive(Deserialize, Serialize, Debug)]
pub struct SuccessMsg {
    msg: String,
}

pub(crate) fn translate_string_to_map(input: String) -> HashMap<String, String> {
    let mut result = HashMap::new();
    let parsed_input = input.split(",").collect::<Vec<_>>();
    for part in parsed_input {
        let subpart = part.split(":").collect::<Vec<_>>();
        let name = subpart[0].split("\"").collect::<Vec<_>>()[1].to_string();
        let value = subpart[1].split("\"").collect::<Vec<_>>()[1].to_string();
        //let name = String::from(name.strip_suffix("\\\"").unwrap());
        log!(Level::Debug, "Name {:?}", name);
        log!(Level::Debug, "Value {:?}", value);

        result.insert(name, value);
    }

    return result;
}

pub(crate) fn get_expression<T>(map: HashMap<String, String>, key: String) -> Expression<T> 
where
    T: Float + std::str::FromStr,
    T::Err: std::fmt::Debug
{
    log!(Level::Debug, "Key {:?}", key);

    if map[&key] == "Number" {
        return Expression::Number {
            number: map[&(key + "/number")].parse::<T>().unwrap()
        }
    } else if map[&key] == "Variable" {
        return Expression::Variable {
            var: map[&(key.clone() + "/variable/var")].clone(),
            owner: map[&(key + "/variable/owner")].clone()
        }
    } else {
        let op = if map[&(key.clone() + "/op")] == "Add" {
            BinaryOp::Add
        } else if map[&(key.clone() + "/op")] == "Mul" {
            BinaryOp::Mul
        } else if map[&(key.clone() + "/op")] == "Div" {
            BinaryOp::Div
        } else {
            BinaryOp::Sub
        };

        return Expression::BinOp {
            left: Box::new(get_expression(map.clone(), key.clone()+"/left")),
            right: Box::new(get_expression(map, key+"/right")),
            op: op,
        }
    }
}

pub(crate) async fn expression(mut req: Request<()>) -> tide::Result<Body> {
    let form_data = req.body_string().await?;
    log!(Level::Debug, "got {:?}", form_data);
    let map = translate_string_to_map(form_data);
    let expr = get_expression::<f32>(map, "expression".to_string());
    log!(Level::Debug, "Value {:?}", expr);
    //log!(Level::Debug, "{:?}", eval_expression(&exp, &HashMap::new()));

    Body::from_json(&SuccessMsg {
        msg: String::from("Nice"),
    })
}
