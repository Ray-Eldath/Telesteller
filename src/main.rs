use tokio::net::TcpListener;
use tokio::sync::{Semaphore, broadcast};
use telesteller::server::Server;
use structopt::StructOpt;
use std::sync::Arc;

#[derive(StructOpt)]
#[structopt()]
struct Opt {
    #[structopt(short, long, default_value = "127.0.0.1:18990")]
    addr: String,
    #[structopt(long, default_value = "40960")]
    max_connection: usize,
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();

    let listener = TcpListener::bind(opt.addr).await.unwrap();
    let (shutdown_tx, _) = broadcast::channel(1);
    let semaphore = Semaphore::new(opt.max_connection);

    let mut server = Server::new(listener, shutdown_tx, Arc::new(semaphore));
    // server.serve().await
}