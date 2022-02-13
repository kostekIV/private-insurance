use crate::rest::expression;
use tide::http::headers::HeaderValue;
use tide::log::LevelFilter;

mod expressions;
mod rest;

use tide::security::{CorsMiddleware, Origin};

fn get_cors() -> CorsMiddleware {
    CorsMiddleware::new()
        .allow_methods("GET, POST, OPTIONS".parse::<HeaderValue>().unwrap())
        .allow_origin(Origin::from("*"))
        .allow_credentials(false)
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    tide::log::with_level(LevelFilter::Debug);
    let mut app = tide::new();

    app.at("/exp").post(expression);

    app.with(get_cors());
    app.listen("127.0.0.1:8080").await?;

    Ok(())
}
