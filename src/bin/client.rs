use h2::client::{self, Builder, SendRequest, Connection};
use http::{Method, Request};
use std::error::Error;
use std::str::Bytes;
use std::{sync::Arc, time::Duration};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::task::JoinSet;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let req_amount = std::env::args()
        .nth(1)
        .expect("Amount of requests shoild be passed")
        .parse::<u16>()
        .expect("Invalid parameter");
    assert!(
        req_amount > 0 && req_amount <= 100,
        "Amount of requests should be between 1 and 100"
    );

    let tcp = TcpStream::connect("127.0.0.1:8080").await?;
    let h2: (SendRequest<&[u8]>, Connection<TcpStream, &[u8]>) = Builder::new()
        .reset_stream_duration(Duration::from_secs(2))
        .max_concurrent_streams(100)
        .handshake(tcp).await?;
    let (h2, connection) = h2;
    // let (h2, connection) = client::handshake(tcp).await?;

    tokio::spawn(async move {
        connection.await.unwrap();
    });

    let h2 = Arc::new(RwLock::new(h2.ready().await?));
    let mut set = JoinSet::new();
    // let aggregator =

    for i in 0..req_amount {
        let h2 = h2.clone();
        set.spawn(async move {
            let request = Request::builder()
                .method(Method::GET)
                .uri("https://www.example.com/")
                .body(())
                .unwrap();
            let end = i == req_amount - 1;
            let (response, _) = h2.write().await.send_request(request, end).unwrap();
            response.await.unwrap()
        });
    }

    while let Some(res) = set.join_next().await {
        res.unwrap();
    }

    Ok(())
}
