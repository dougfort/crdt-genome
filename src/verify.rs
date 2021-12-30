use hyper::{body, Client};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::watch;

use crate::config;
use crate::state;

/// peridically poll the other actors for their current genome
/// uses HTTP GET /genome
/// reports a count of the number of matchig genomes
pub async fn verifier(
    state: Arc<RwLock<state::State>>,
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
