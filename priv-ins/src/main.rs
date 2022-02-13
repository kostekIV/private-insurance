#[macro_use]
extern crate ff;
extern crate futures;
extern crate tokio;

use tide::{
    http::headers::HeaderValue,
    security::{CorsMiddleware, Origin},
};

mod crypto;
mod expressions;
mod protocol;
mod rest;

use crate::rest::expression;

fn get_cors() -> CorsMiddleware {
    CorsMiddleware::new()
        .allow_methods("GET, POST, OPTIONS".parse::<HeaderValue>().unwrap())
        .allow_origin(Origin::from("*"))
        .allow_credentials(false)
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    tide::log::start();
    let mut app = tide::new();

    app.at("/exp").post(expression);

    app.with(get_cors());
    app.listen("127.0.0.1:8080").await?;

    Ok(())
}
