use std::collections::HashMap;
use crate::expressions::{eval_expression, Expression};
use tide::{Body, Request};

use serde::{Deserialize, Serialize};
use tide::log::{log, Level};

#[derive(Deserialize, Serialize, Debug)]
pub struct SuccessMsg {
    msg: String,
}

pub(crate) async fn expression(mut req: Request<()>) -> tide::Result<Body> {
    let exp: Expression<f64> = req.body_json().await?;

    log!(Level::Debug, "got {:?}", exp);
    log!(Level::Debug, "{:?}", eval_expression(&exp, &HashMap::new()));

    Body::from_json(&SuccessMsg {
        msg: String::from("Nice"),
    })
}
