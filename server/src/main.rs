use poem::{listener::TcpListener, Server};

use dodatok::config::Config;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();
    let config = Config::from_file("config.toml");
    Server::new(TcpListener::bind("0.0.0.0:5000"))
        .run(dodatok::create_app(config).await)
        .await
}
