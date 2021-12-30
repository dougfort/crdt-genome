use hyper::{Body, Client, Method, Request};
use rand::Rng;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::watch;

use crate::config;
use crate::state;

/// an async function that periodically mutates the genome
/// circulates CmRDT Op notification to the other actors
/// using HTTP Post /genome
pub async fn mutator(
    state: Arc<RwLock<state::State>>,
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
