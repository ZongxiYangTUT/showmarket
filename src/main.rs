use axum::Router;
use std::collections::HashMap;
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
        // 本地模拟三个 A 股标的的实时价格。
        let symbols = vec![
            "600000.SH".to_string(),
            "000001.SZ".to_string(),
            "300750.SZ".to_string(),
        ];
        let mut last_prices: HashMap<String, f64> = HashMap::new();
        let mut interval = tokio::time::interval(Duration::from_millis(800));

        loop {
            interval.tick().await;
            for sym in &symbols {
                let prev = last_prices.get(sym).cloned();
                let update = svc.next_mock_price(sym, prev);
                last_prices.insert(sym.clone(), update.price);
                state.set_latest(update).await;
            }
        }
    });
}
