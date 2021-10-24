use anyhow::Error;
use axum::{
    extract::Extension,
    handler::{get, post},
    Json, Router,
};
use crdts::list;
use hyper::{body, Body, Client, Method, Request};
use rand::Rng;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::Notify;
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
    tracing_subscriber::fmt::init();

    let config = config::load_configuration()?;

    tracing::info!(
        "actor = {}; count = {}, base port = {}",
        config.actor_id,
        config.actor_count,
        config.base_port_number
    );

    let state = SharedState::default();

    let mutator_notify = Arc::new(Notify::new());
    let mutator_notify2 = Arc::clone(&mutator_notify);

    let mutator_state = state.clone();
    let mutator_handle = tokio::spawn(async move {
        mutator(mutator_state, config, mutator_notify2).await;
    });

    let verifier_notify = Arc::new(Notify::new());
    let verifier_notify2 = Arc::clone(&verifier_notify);

    let verifier_state = state.clone();
    let verifier_handle = tokio::spawn(async move {
        verifier(verifier_state, config, verifier_notify2).await;
    });

    // build our application
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

    mutator_notify.notify_one();
    let join_result = mutator_handle.await?;
    tracing::debug!("mutator join result = {:?}", join_result);

    verifier_notify.notify_one();
    let join_result = verifier_handle.await?;
    tracing::debug!("verifier join result = {:?}", join_result);

    Ok(())
}

async fn say_hello() -> String {
    "Hello, World!\n".to_string()
}

async fn update_genome(
    Json(op): Json<list::Op<Gene, Actor>>,
    Extension(state): Extension<SharedState>,
) {
    state.write().unwrap().genome.apply(op);
}

async fn get_genome(Extension(state): Extension<SharedState>) -> String {
    format!("{}", state.read().unwrap().genome)
}

/// mutator is an async function that periodically mutates the genome
/// mutator broadcasts CmRDT Op notification to the other Actors
async fn mutator(
    state: Arc<RwLock<State>>,
    config: config::Config,
    mutator_notify: Arc<tokio::sync::Notify>,
) {
    // wait for the server to start
    // TODO: #3 retry connection
    tokio::time::sleep(Duration::from_secs(5)).await;

    let mut more = true;
    while more {
        let op = {
            let mut lock = state.write().unwrap();
            lock.genome.generate(config.actor_id)
        };
        let op_string = serde_json::to_string(&op).unwrap();
        for i in 0..config.actor_count {
            if i != config.actor_id {
                let op_string = op_string.clone();
                tokio::spawn(async move {
                    let port_number = config.base_port_number + i;
                    let uri = format!("http://127.0.0.1:{}/genome", port_number);
                    let req = Request::builder()
                        .method(Method::POST)
                        .uri(uri.clone())
                        .header("content-type", "application/json")
                        .body(Body::from(op_string))
                        .unwrap();
                    let client = Client::new();
                    let resp = client.request(req).await.unwrap();
                    tracing::debug!("POST {}; Response: {}", uri, resp.status());
                });
            }
        }
        let sleep_interval = {
            let mut rng = rand::thread_rng();
            Duration::from_secs(rng.gen_range(0..20))
        };
        tokio::select! {
            _ = tokio::time::sleep(sleep_interval) => {}
            _ = mutator_notify.notified() => {more = false}
        }
    }
}

/// verifier is an async function that polls the other Actors for their
/// current genome and compares them to the local genome
async fn verifier(
    state: Arc<RwLock<State>>,
    config: config::Config,
    verifier_notify: Arc<tokio::sync::Notify>,
) {
    // wait for the server to start
    // TODO: #3 retry connection
    tokio::time::sleep(Duration::from_secs(5)).await;

    let mut more = true;
    while more {
        let mut match_count = 0;
        for i in 0..config.actor_count {
            if i != config.actor_id {
                let port_number = config.base_port_number + i;
                let uri: hyper::Uri = format!("http://127.0.0.1:{}/genome", port_number)
                    .parse()
                    .unwrap();
                let client = Client::new();
                let resp = client.get(uri.clone()).await.unwrap();
                tracing::debug!("GET {}; Response: {}", uri, resp.status());
                let bytes = body::to_bytes(resp.into_body()).await.unwrap();
                let remote_genome_str =
                    String::from_utf8(bytes.to_vec()).expect("response was not valid utf-8");
                let local_genome_str = state.read().unwrap().genome.to_string();
                if local_genome_str == remote_genome_str {
                    match_count += 1;
                }
            }
        }
        tracing::debug!("match count = {}", match_count);
        let sleep_interval = Duration::from_secs(5);
        tokio::select! {
            _ = tokio::time::sleep(sleep_interval) => {}
            _ = verifier_notify.notified() => {more = false}
        };
    }
}

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

#[cfg(windows)]
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("faild to install CTRL+C handler");
    tracing::info!("signal received, starting graceful shutdown")
}
