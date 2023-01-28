use clap::Parser;
use poem::{listener::TcpListener, Server};

use dodatok::config::Config;

#[derive(Parser)]
struct Args {
    #[clap(short, long, default_value = "config.toml")]
    config: String,

    #[clap(short, long, default_value_t = 5000)]
    port: u16,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let config = Config::from_file(&args.config);

    Server::new(TcpListener::bind(("0.0.0.0", args.port)))
        .run(dodatok::create_app(config).await)
        .await
}
