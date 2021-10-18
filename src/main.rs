use axum::{
    handler::get,
    Router,
};
use anyhow::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let handle = tokio::spawn(async move {

    });

    // build our application with a single route
    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse()?)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    let join_result = handle.await?;
    println!("join result = {:?}", join_result);
 
    Ok(())
}

#[cfg(unix)]
pub async fn shutdown_signal() {
    use std::io;
    use tokio::signal::unix::SignalKind;

    async fn terminate() -> io::Result<()> {
        tokio::signal::unix::signal(SignalKind::terminate())?
            .recv()
            .await;
        Ok(())
    }

    tokio::select! {
        _ = terminate() => {},
        _ = tokio::signal::ctrl_c() => {},
    }
    println!("signal received, starting graceful shutdown")
}

#[cfg(windows)]
pub async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("faild to install CTRL+C handler");
    println!("signal received, starting graceful shutdown")
}
