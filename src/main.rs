use axum::Router;
use std::net::SocketAddr;
use std::time::Duration;
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
    let svc = showmarket::services::ashare::AshareService::new();
    tokio::spawn(async move {
        // 先只推送上证指数，避免价格在多个标的之间来回闪烁。
        let symbols = vec!["000001.SH".to_string()];
        let mut interval = tokio::time::interval(Duration::from_millis(800));

        loop {
            interval.tick().await;
            for sym in &symbols {
                match svc.fetch_realtime_quote(sym).await {
                    Ok(update) => {
                        state.set_latest(update).await;
                    }
                    Err(err) => {
                        tracing::warn!(%sym, error = %err, "failed to fetch realtime quote");
                    }
                }
            }
        }
    });
}
