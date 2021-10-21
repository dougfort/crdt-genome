use anyhow::Error;
use axum::{
    extract::Extension,
    handler::{get, post},
    Json, Router,
};
use crdts::list;
use hyper::{Body, Client, Method, Request};
use rand::Rng;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::Notify;
use tower::ServiceBuilder;
use tower_http::{add_extension::AddExtensionLayer, trace::TraceLayer};

mod genome;
use genome::{Actor, Gene};

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

    let actor_id: usize = 8888;
    let state = SharedState::default();

    let mutator_notify = Arc::new(Notify::new());
    let mutator_notify2 = Arc::clone(&mutator_notify);

    let mutator_state = state.clone();
    let mutator_handle = tokio::spawn(async move {
        mutator(mutator_state, actor_id, mutator_notify2).await;
    });

    // build our application with a single route
    let app = Router::new()
        .route("/", get(say_hello))
        .route("/genome", post(update_genome))
        .layer(TraceLayer::new_for_http())
        .layer(ServiceBuilder::new().layer(AddExtensionLayer::new(state)));

    // run it with hyper
    let addr = "0.0.0.0:3000".parse()?;
    tracing::debug!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    mutator_notify.notify_one();
    let join_result = mutator_handle.await?;
    tracing::debug!("mutator join result = {:?}", join_result);

    Ok(())
}

async fn say_hello() -> String {
    "Hello, World!".to_string()
}

async fn update_genome(
    Json(op): Json<list::Op<Gene, Actor>>,
    Extension(state): Extension<SharedState>,
) {
    tracing::debug!("server received op: {:?}", op);
    state.write().unwrap().genome.apply(op);
}

async fn mutator(
    state: Arc<RwLock<State>>,
    actor: usize,
    mutator_notify: Arc<tokio::sync::Notify>,
) {
    // wait for the server to start
    tokio::time::sleep(Duration::from_secs(5)).await;

    let mut more = true;
    while more {
        let op = {
            let item: u8 = 43;
            let mut lock = state.write().unwrap();
            tracing::debug!("actor: {}; appending {}", actor, item);
            lock.genome.append(item, actor)
        };
        let op_string = serde_json::to_string(&op).unwrap();
        let req = Request::builder()
            .method(Method::POST)
            .uri("http://127.0.0.1:3000/genome")
            .header("content-type", "application/json")
            .body(Body::from(op_string))
            .unwrap();
        let client = Client::new();
        let resp = client.request(req).await.unwrap();
        tracing::debug!("/genome Response: {}", resp.status());
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
