use std::sync::Arc;

use structopt::StructOpt;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, Semaphore};
use tracing::{info, Level};

use telesteller::server::Server;

#[derive(StructOpt, Debug)]
#[structopt()]
struct Opt {
    #[structopt(short, long, default_value = "127.0.0.1:18990")]
    addr: String,
    #[structopt(long, default_value = "40960")]
    max_connection: usize,
    #[structopt(long, default_value = "info")]
    log_filter: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let opt = Opt::from_args();
    let filter = tracing_subscriber::EnvFilter::try_new(&opt.log_filter)?;
    tracing_subscriber::fmt().with_env_filter(filter).pretty().try_init()?;

    let listener = TcpListener::bind(&opt.addr).await?;
    let (shutdown_tx, _) = broadcast::channel(1);
    let semaphore = Semaphore::new(opt.max_connection);

    let mut server = Server::new(listener, shutdown_tx, Arc::new(semaphore));
    server.serve().await;

    Ok(())
}