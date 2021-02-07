use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt()]
pub struct Opt {
    #[structopt(short, long, default_value = "127.0.0.1:18990")]
    pub addr: String,
    #[structopt(long, default_value = "40960")]
    pub max_connection: usize,
    #[structopt(long)]
    pub max_session: Option<usize>,
    #[structopt(long, default_value = "info")]
    pub log_filter: String,
}