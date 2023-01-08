use poem::{listener::TcpListener, Server};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    Server::new(TcpListener::bind("0.0.0.0:5000"))
        .run(dodatok::create_app())
        .await
}
