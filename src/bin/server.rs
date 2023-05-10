use h2::server;
use http::{Response, StatusCode};
use rand::Rng;
use std::collections::VecDeque;
use std::process;
use std::sync::{
    atomic::{AtomicU16, AtomicU64, AtomicU8, Ordering},
    Arc,
};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio::task::JoinSet;
use tokio::time::{sleep, Duration, Instant};

const MAX_CONNECTIONS: usize = 5;

#[derive(Default)]
struct Aggregator {
    counter: AtomicU16,
    max_ts: AtomicU64,
    min_ts: AtomicU64,
    sum_ts: AtomicU64,
}

impl Aggregator {
    pub fn new() -> Self {
        Self {
            // 500 is the maximum time spend
            min_ts: AtomicU64::new(500),
            ..Default::default()
        }
    }
    pub fn add_connection(&self, time: u64) {
        self.counter.fetch_add(1, Ordering::SeqCst);
        self.max_ts.fetch_max(time, Ordering::SeqCst);
        self.min_ts.fetch_min(time, Ordering::SeqCst);
        self.sum_ts.fetch_add(time, Ordering::SeqCst);
    }

    pub fn print_res(&self, header: &str) {
        let counter = self.counter.load(Ordering::SeqCst);
        let avg = if counter > 0 {
            self.sum_ts.load(Ordering::SeqCst) / counter as u64
        } else {
            0
        };
        println!("{}", header);
        println!("Received requests: {}", counter);
        println!("Max time handle: {} ms", self.max_ts.load(Ordering::SeqCst));
        println!("Min time handle: {} ms", self.min_ts.load(Ordering::SeqCst));
        println!("Average time handle: {} ms", avg);
        println!("--------------------------");
    }
}

struct ThreadData {
    max_threads: usize,
    active_threads: AtomicU8,
    queue: VecDeque<TcpStream>,
}

impl ThreadData {
    pub fn new(max_threads: usize) -> Self {
        Self {
            max_threads,
            active_threads: AtomicU8::new(0),
            // Probably there is a better way, but in this case we restrict queue to be 20 items
            queue: VecDeque::with_capacity(20),
        }
    }

    pub fn is_full(&self) -> bool {
        self.active_threads.load(Ordering::SeqCst) == self.max_threads as u8
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize listener and total storage
    let listener = Arc::new(TcpListener::bind("127.0.0.1:8080").await?);
    let pool = Arc::new(RwLock::new(ThreadData::new(MAX_CONNECTIONS)));
    let global_aggregator: Arc<Aggregator> = Arc::new(Aggregator::new());
    let mut set = JoinSet::new();
    let ga = global_aggregator.clone();
    ctrlc::set_handler(move || {
        ga.print_res("Closed server");
        process::exit(0x0);
    })
    .expect("Error setting Ctrl-C handler");
    loop {
        let pool = pool.clone();
        let global_aggregator = global_aggregator.clone();
        match listener.accept().await {
            Ok((upcoming_stream, _)) => {
                // accepting all requests
                // could be optimised by skipping push/pop ops on empty queue
                pool.write().await.queue.push_back(upcoming_stream);
                // handling them in FILO order
                if !pool.read().await.is_full() {
                    // queue is never epmty on this stage
                    let socket = pool.write().await.queue.pop_front().unwrap();
                    pool.write()
                        .await
                        .active_threads
                        .fetch_add(1, Ordering::SeqCst);
                    set.spawn(async move {
                        let start = Instant::now();
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
                                        aggregator.add_connection(timeout);
                                        // Send the response back to the client
                                        respond.send_response(response, true).unwrap();
                                    });
                                }
                                Err(_err) => break,
                            }
                        }
                        aggregator.print_res("Closed client connection");
                        pool.write()
                            .await
                            .active_threads
                            .fetch_sub(1, Ordering::SeqCst);
                        let end = start.elapsed();
                        global_aggregator.add_connection(end.as_millis().try_into().unwrap())
                    });
                }
            }
            Err(err) => println!("Cannot hanlde client with err: {}", err),
        }
    }
}
