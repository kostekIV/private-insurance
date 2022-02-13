use crate::expressions::{BinaryOp, Expression};
use crate::protocol::run_nodes;
use num_traits::Num;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs};
use tide::log::{log, Level};
use tide::{Body, Request};

#[derive(Deserialize, Serialize, Debug)]
pub struct SuccessMsg {
    msg: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct VariableConfig {
    nodes: Vec<HashMap<String, u64>>,
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

pub(crate) fn get_expression<T>(
    map: HashMap<String, String>,
    key: String,
    pairing: &mut HashMap<String, String>,
) -> Expression<T>
where
    T: Num + std::str::FromStr,
    T::Err: std::fmt::Debug,
{
    log!(Level::Debug, "Key {:?}", key);

    if map[&key] == "Number" {
        return Expression::Number {
            number: map[&(key + "/number")].parse::<T>().unwrap(),
        };
    } else if map[&key] == "Variable" {
        let name = map[&(key.clone() + "/variable/var")].clone();
        let owner = map[&(key + "/variable/owner")].clone();
        pairing.insert(name.clone(), owner);

        return Expression::Variable { name: name };
    } else {
        let op = if map[&(key.clone() + "/op")] == "Sum" {
            BinaryOp::Add
        } else if map[&(key.clone() + "/op")] == "Mul" {
            BinaryOp::Mul
        } else if map[&(key.clone() + "/op")] == "Div" {
            BinaryOp::Div
        } else {
            BinaryOp::Sub
        };

        return Expression::BinOp {
            left: Box::new(get_expression(map.clone(), key.clone() + "/left", pairing)),
            right: Box::new(get_expression(map, key + "/right", pairing)),
            op: op,
        };
    }
}

pub(crate) async fn expression(mut req: Request<()>) -> tide::Result<Body> {
    let form_data = req.body_string().await?;
    log!(Level::Debug, "got {:?}", form_data);
    let map = translate_string_to_map(form_data);
    let mut pairing = HashMap::new();
    let n_parties: u32 = map
        .get(&String::from("amount_of_people"))
        .unwrap()
        .parse()
        .unwrap();
    let expr = get_expression::<u64>(map, "expression".to_string(), &mut pairing);

    let variable_config: VariableConfig = serde_json::from_str(
        &fs::read_to_string("variables_config.json")
            .expect("Unable to read config file containing peer addresses"),
    )
    .expect("JSON was not well-formatted");
    println!("{:?}", variable_config);

    run_nodes(n_parties, variable_config.nodes, expr).await;

    Body::from_json(&SuccessMsg {
        msg: String::from("Nice"),
    })
}
