use futures::{stream, StreamExt};
use rand::{thread_rng, Rng};
use std::time::{Duration};

const UPPER_BOUND: u64 = 200;
const INSTANCE_COUNT: usize = 20;
const SLEEP_UPPER_BOUND: u64 = 20;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    stream::iter(0..UPPER_BOUND)
        .for_each_concurrent(INSTANCE_COUNT, |number| async move {
            let mut rng = thread_rng();
            let sleep_ms: u64 = rng.gen_range(0..SLEEP_UPPER_BOUND);
            tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
            println!("{}", number);
        })
        .await;
}
