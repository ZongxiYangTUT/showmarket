use axum::Router;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "showmarket=info,tower_http=info,axum=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = showmarket::state::AppState::new();
    spawn_binance_price_task(state.clone());

    let app: Router = showmarket::app(state);
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("listening on {}", addr);
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

fn spawn_binance_price_task(state: showmarket::state::AppState) {
    let svc = showmarket::services::binance::BinancePriceService::new();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
        loop {
            interval.tick().await;
            match svc.fetch_btcusdt().await {
                Ok(update) => {
                    state.set_latest(update).await;
                }
                Err(err) => {
                    tracing::warn!(error = %err, "failed to fetch BTCUSDT");
                }
            }
        }
    });
}
