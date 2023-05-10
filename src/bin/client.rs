use h2::client;
use http::{Method, Request};
use std::error::Error;
use std::net::TcpStream;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::{sync::Arc, time::Duration};
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
    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let tcp = TcpStream::connect_timeout(&socket, Duration::from_secs(2))?;
    tcp.set_write_timeout(Some(Duration::from_secs(2)))?;
    tcp.set_read_timeout(Some(Duration::from_secs(2)))?;
    let tokio_stream = tokio::net::TcpStream::from_std(tcp)?;
    let (h2, connection) = client::handshake(tokio_stream).await?;
    tokio::spawn(async move {
        connection.await.unwrap();
    });
    let h2 = Arc::new(RwLock::new(h2.ready().await?));
    let mut set = JoinSet::new();

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
