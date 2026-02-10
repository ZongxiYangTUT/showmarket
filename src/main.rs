use anyhow::Ok;
use axum::{Router, response::Redirect, routing::get};
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/hello-world", get(hello_world))
        .fallback(anything_else);
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

async fn hello_world() -> &'static str {
    "Hello, World!"
}

async fn anything_else() -> Redirect {
    Redirect::to("/hello-world")
}
