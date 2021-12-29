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
use hyper::{body, Body, Client, Method, Request};
use rand::Rng;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::watch;
use tower::ServiceBuilder;
use tower_http::{add_extension::AddExtensionLayer, trace::TraceLayer};

mod genome;
use genome::{Actor, Gene};

mod config;

type SharedState = Arc<RwLock<State>>;

#[derive(Default)]
struct State {
    genome: genome::Genome,
}

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

    let state = SharedState::default();

    let mut halt = false;
    let (halt_tx, halt_rx) = watch::channel(halt);

    let mutator_state = state.clone();
    let mutatator_halt_rx = halt_rx.clone();
    let mutator_handle = tokio::spawn(async move {
        mutator(mutator_state, config, mutatator_halt_rx).await;
    });

    let verifier_state = state.clone();
    let verifier_halt_rx = halt_rx.clone();
    let verifier_handle = tokio::spawn(async move {
        verifier(verifier_state, config, verifier_halt_rx).await;
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
    Extension(state): Extension<SharedState>,
) {
    state.write().unwrap().genome.apply(op);
}

/// HTTP handler for GET /genome
/// returns a string representation of the genome
async fn get_genome(Extension(state): Extension<SharedState>) -> String {
    format!("{}", state.read().unwrap().genome)
}

/// an async function that periodically mutates the genome
/// circulates CmRDT Op notification to the other actors
/// using HTTP Post /genome
async fn mutator(
    state: Arc<RwLock<State>>,
    config: config::Config,
    mut halt_rx: watch::Receiver<bool>,
) {
    let mut halt = *halt_rx.borrow();
    while !halt {
        let op = {
            let mut lock = state.write().unwrap();
            lock.genome.generate(config.actor_id)
        };
        let op_string = serde_json::to_string(&op).unwrap();
        for i in 0..config.actor_count {
            if i != config.actor_id {
                let port_number = config.base_port_number + i;
                let op_string = op_string.clone();
                tokio::spawn(send_mutation_to_actor(port_number, op_string));
            }
        }
        let sleep_interval = {
            let mut rng = rand::thread_rng();
            Duration::from_secs(rng.gen_range(0..20))
        };
        tokio::select! {
            _ = tokio::time::sleep(sleep_interval) => {}
            _ = halt_rx.changed() => {halt = *halt_rx.borrow()}
        }
    }
}

async fn send_mutation_to_actor(port_number: usize, op_string: String) {
    let uri = format!("http://127.0.0.1:{}/genome", port_number);
    let req = Request::builder()
        .method(Method::POST)
        .uri(uri.clone())
        .header("content-type", "application/json")
        .body(Body::from(op_string))
        .unwrap();
    let client = Client::new();
    match client.request(req).await {
        Ok(resp) => {
            tracing::debug!("POST {}; Response: {}", uri, resp.status());
        }
        Err(e) => {
            tracing::error!("POST Failed {}", e);
        }
    }
}

/// peridically poll the other actors for their current genome
/// uses HTTP GET /genome
/// reports a count of the number of matchig genomes
async fn verifier(
    state: Arc<RwLock<State>>,
    config: config::Config,
    mut halt_rx: watch::Receiver<bool>,
) {
    let mut halt = *halt_rx.borrow();
    while !halt {
        let mut join_handles = vec![];
        for i in 0..config.actor_count {
            if i != config.actor_id {
                let port_number = config.base_port_number + i;
                let join_handle = poll_actor_genome(port_number);
                join_handles.push((i, join_handle));
            }
        }

        let local_genome_str = state.read().unwrap().genome.to_string();
        let mut match_count = 0;
        for (_actor_id, join_handle) in join_handles {
            let remote_genome_str = join_handle.await;
            if local_genome_str == remote_genome_str {
                match_count += 1;
            }
        }
        tracing::info!("match count = {}", match_count);

        let sleep_interval = Duration::from_secs(5);
        tokio::select! {
            _ = tokio::time::sleep(sleep_interval) => {}
            _ = halt_rx.changed() => {halt = *halt_rx.borrow()}
        };
    }
}

async fn poll_actor_genome(port_number: usize) -> String {
    let uri: hyper::Uri = format!("http://127.0.0.1:{}/genome", port_number)
        .parse()
        .unwrap();
    let client = Client::new();
    match client.get(uri.clone()).await {
        Ok(resp) => {
            tracing::debug!("GET {}; Response: {}", uri, resp.status());
            let bytes = body::to_bytes(resp.into_body()).await.unwrap();
            String::from_utf8(bytes.to_vec()).expect("response was not valid utf-8")
        }
        Err(e) => {
            tracing::error!("GET failed: {}", e);
            "".to_string()
        }
    }
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
