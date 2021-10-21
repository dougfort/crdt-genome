use axum::{
    handler::{get, post},
    extract::Extension,
    Router,
    Json,
};
use anyhow::Error;
use rand::Rng;
use std::time::{Duration};
use tokio::sync::Notify;
use std::sync::{Arc, RwLock};
use tower::ServiceBuilder;
use tower_http::add_extension::AddExtensionLayer;
use hyper::{Body, Method, Request, Client};
use crdts::list;

mod genome;
use genome::{Gene, Actor};

type SharedState = Arc<RwLock<State>>;

#[derive(Default)]
struct State {
    genome: genome::Genome,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
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
    .layer(
            ServiceBuilder::new()
                .layer(AddExtensionLayer::new(state))
        );

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse()?)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    mutator_notify.notify_one();
    let join_result = mutator_handle.await?;
    println!("join result = {:?}", join_result);
 
    Ok(())
}

async fn say_hello() -> String {
    "Hello, World!".to_string()    
}

async fn update_genome(
    Json(op): Json<list::Op::<Gene, Actor>>,
    Extension(state): Extension<SharedState>,
) {
    println!("server received op: {:?}", op);
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
            println!("actor: {}; appending {}", actor, item); 
            lock.genome.append(item, actor)   
        };
        let op_string = serde_json::to_string(&op).unwrap();
        println!("op = {:?}", op);
        let req = Request::builder()
            .method(Method::POST)
            .uri("http://127.0.0.1:3000/genome")
            .header("content-type", "application/json")
            .body(Body::from(op_string)).unwrap();
        let client = Client::new();
        let resp = client.request(req).await.unwrap();            
        println!("Response: {}", resp.status());
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
    println!("signal received, starting graceful shutdown")
}

#[cfg(windows)]
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("faild to install CTRL+C handler");
    println!("signal received, starting graceful shutdown")
}
