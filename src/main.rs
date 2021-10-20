use axum::{
    handler::get,
    Router,
};
use anyhow::Error;
use rand::Rng;
use std::time::{Duration};
use tokio::sync::Notify;
use std::sync::Arc;

mod genome;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mutator_notify = Arc::new(Notify::new());
    let mutator_notify2 = mutator_notify.clone();

    let handle = tokio::spawn(async move {
        let mut more = true;
        while more {
            let sleep_interval = {
                let mut rng = rand::thread_rng();
                Duration::from_secs(rng.gen_range(0..20))
            };
            println!("sleep: {:?}", sleep_interval);    
            tokio::select! {
                _ = tokio::time::sleep(sleep_interval) => {}
                _ = mutator_notify2.notified() => {more = false}
            }            
        }
    });

    // build our application with a single route
    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse()?)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

        mutator_notify.notify_one();
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
