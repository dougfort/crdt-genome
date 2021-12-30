#![warn(missing_docs)]

//! crdts-genome
//! Experiments with Rust CRDTs using Tokio web application framework Axum.

use anyhow::Error;
use axum::{
    extract::Extension,
    routing::{get, post},
    Json, Router,
};
use crdts::list;
use tokio::sync::watch;
use tower::ServiceBuilder;
use tower_http::{add_extension::AddExtensionLayer, trace::TraceLayer};

mod genome;
use genome::{Actor, Gene};

mod config;
mod mutate;
mod state;
mod verify;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Set the RUST_LOG, if it hasn't been explicitly defined
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "crdt_genome=debug,tower_http=debug")
    }
    tracing_subscriber::fmt()
        .with_env_filter("crdt_genome=debug,tower_http=debug")
        .init();

    let config = config::load_configuration()?;

    tracing::info!(
        "actor = {}; count = {}, base port = {}",
        config.actor_id,
        config.actor_count,
        config.base_port_number
    );

    let state = state::SharedState::default();

    let mut halt = false;
    let (halt_tx, halt_rx) = watch::channel(halt);

    let mutator_state = state.clone();
    let mutatator_halt_rx = halt_rx.clone();
    let mutator_handle = tokio::spawn(async move {
        mutate::mutator(mutator_state, config, mutatator_halt_rx).await;
    });

    let verifier_state = state.clone();
    let verifier_halt_rx = halt_rx.clone();
    let verifier_handle = tokio::spawn(async move {
        verify::verifier(verifier_state, config, verifier_halt_rx).await;
    });

    // build our application
    tracing::info!("build application");
    let app = Router::new()
        .route("/", get(say_hello))
        .route("/genome", post(update_genome))
        .route("/genome", get(get_genome))
        .layer(TraceLayer::new_for_http())
        .layer(ServiceBuilder::new().layer(AddExtensionLayer::new(state)));

    // run it with hyper
    let port_number = config.base_port_number + config.actor_id;
    let addr = format!("0.0.0.0:{}", port_number).parse()?;
    tracing::debug!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    halt = true;
    halt_tx.send(halt)?;

    let join_result = mutator_handle.await?;
    tracing::debug!("mutator join result = {:?}", join_result);

    let join_result = verifier_handle.await?;
    tracing::debug!("verifier join result = {:?}", join_result);

    Ok(())
}

/// HTTP handler for GET /
async fn say_hello() -> String {
    "Hello, World!\n".to_string()
}

/// HTTP handler for POST /genome
/// Request body must contain a JSON representation of a CmRDT Op object
async fn update_genome(
    Json(op): Json<list::Op<Gene, Actor>>,
    Extension(state): Extension<state::SharedState>,
) {
    state.write().unwrap().genome.apply(op);
}

/// HTTP handler for GET /genome
/// returns a string representation of the genome
async fn get_genome(Extension(state): Extension<state::SharedState>) -> String {
    format!("{}", state.read().unwrap().genome)
}

/// unix signal handler
/// pasted from Axum exampe
#[cfg(unix)]
async fn shutdown_signal() {
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
    tracing::info!("signal received, starting graceful shutdown")
}

/// windows signal handler
/// pasted from Axum exampe
#[cfg(windows)]
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("faild to install CTRL+C handler");
    tracing::info!("signal received, starting graceful shutdown")
}
