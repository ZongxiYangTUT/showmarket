use axum::Router;
use futures_util::StreamExt;
use serde::Deserialize;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;
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
    tokio::spawn(async move {
        loop {
            let ws_url = "wss://stream.binance.com:9443/ws/btcusdt@trade";
            match connect_async(ws_url).await {
                Ok((ws_stream, _)) => {
                    tracing::info!("connected to Binance trade stream");
                    let (_write, mut read) = ws_stream.split();

                    while let Some(msg) = read.next().await {
                        let Ok(msg) = msg else {
                            break;
                        };
                        if !msg.is_text() {
                            continue;
                        }
                        let Ok(text) = msg.into_text() else {
                            continue;
                        };
                        if let Ok(trade) = serde_json::from_str::<BinanceTrade>(&text) {
                            let price = trade.p.parse::<f64>().unwrap_or(0.0);
                            let update = showmarket::models::price::PriceUpdate {
                                symbol: "BTCUSDT".to_string(),
                                price,
                                ts_ms: trade.e as i64,
                            };
                            state.set_latest(update).await;
                        }
                    }
                    tracing::warn!("Binance trade stream closed, will reconnect");
                }
                Err(err) => {
                    tracing::warn!(error = %err, "failed to connect Binance trade stream");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
            // 短暂等待后重连，避免高频重试
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
}

#[derive(Debug, Deserialize)]
struct BinanceTrade {
    /// price as string
    p: String,
    /// event time (ms)
    e: u64,
}
