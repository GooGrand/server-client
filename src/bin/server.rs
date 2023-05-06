use h2::server;
use http::{Response, StatusCode};
use rand::Rng;
use std::sync::{
    atomic::{AtomicU16, AtomicU32, AtomicU8, Ordering},
    Arc,
};
use std::{process, sync::RwLock};
use tokio::net::TcpListener;
use tokio::task::JoinSet;
use tokio::time::{sleep, Duration};

const MAX_CONNECTIONS: usize = 5;

#[derive(Default)]
struct Aggregator {
    counter: AtomicU16,
    max_ts: AtomicU32,
    min_ts: AtomicU32,
    sum_ts: AtomicU32,
}

impl Aggregator {
    pub fn new() -> Self {
        Self {
            // 500 is the maximum time spend
            min_ts: AtomicU32::new(500),
            ..Default::default()
        }
    }
    pub fn add_connection(&self, time: u32) {
        self.counter.fetch_add(1, Ordering::SeqCst);
        self.max_ts.fetch_max(time, Ordering::SeqCst);
        self.min_ts.fetch_min(time, Ordering::SeqCst);
        self.sum_ts.fetch_add(time, Ordering::SeqCst);
    }

    pub fn print_res(&self) {
        let counter = self.counter.load(Ordering::SeqCst);
        let avg = if counter > 0 {
            self.sum_ts.load(Ordering::SeqCst) / counter as u32
        } else {
            0
        };
        println!("--------------------------");
        println!("Finished client connection");
        println!("Received requests: {}", counter);
        println!("Max time handle: {}", self.max_ts.load(Ordering::SeqCst));
        println!("Min time handle: {}", self.min_ts.load(Ordering::SeqCst));
        println!("Average time handle: {}", avg);
    }
}

struct ThreadData {
    max_threads: usize,
    active_threads: AtomicU8,
}

impl ThreadData {
    pub fn new(max_threads: usize) -> Self {
        Self {
            max_threads,
            active_threads: AtomicU8::new(0),
        }
    }

    pub fn is_full(&self) -> bool {
        self.active_threads.load(Ordering::SeqCst) == self.max_threads as u8
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize listener and total storage
    let listener = Arc::new(RwLock::new(TcpListener::bind("127.0.0.1:8080").await?));
    let pool = Arc::new(ThreadData::new(MAX_CONNECTIONS));
    let mut set = JoinSet::new();
    let mut queue = Vec::new();
    ctrlc::set_handler(move || {
        println!("Fuck you");
        process::exit(0x0);
    })
    .expect("Error setting Ctrl-C handler");
    loop {
        let pool = pool.clone();
        match listener.read().unwrap().accept().await {
            Ok((socket, _)) => {
                if pool.is_full() {
                    pool.active_threads.fetch_add(1, Ordering::SeqCst);
                    set.spawn(async move {
                        let mut h2 = server::handshake(socket).await.unwrap();
                        let aggregator: Arc<Aggregator> = Arc::new(Aggregator::new());
                        let mut set = JoinSet::new();
                        while let Some(request) = h2.accept().await {
                            let aggregator = aggregator.clone();
                            match request {
                                Ok((_, mut respond)) => {
                                    set.spawn(async move {
                                        let aggregator = aggregator.clone();
                                        let response = Response::builder()
                                            .status(StatusCode::OK)
                                            .body(())
                                            .unwrap();
                                        let timeout = {
                                            let mut rng = rand::thread_rng();
                                            rng.gen_range(100..500)
                                        };
                                        sleep(Duration::from_millis(timeout)).await;
                                        aggregator.add_connection(timeout as u32);
                                        // Send the response back to the client
                                        respond.send_response(response, true).unwrap();
                                    });
                                }
                                Err(err) => break,
                            }
                        }
                        aggregator.print_res();
                        pool.active_threads.fetch_sub(1, Ordering::SeqCst);
                        aggregator
                    });
                } else {
                    queue.push(socket);
                }
            }
            Err(err) => println!("Cannot hanlde client with err: {}", err),
        }
    }
}
