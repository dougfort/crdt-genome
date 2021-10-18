use axum::{
    handler::get,
    Router,
};
use anyhow::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // build our application with a single route
    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
