use axum::{Json, extract::Path, http::StatusCode, response::IntoResponse};

use crate::services::binance::BinancePriceService;

#[derive(serde::Deserialize)]
pub struct KlineQuery {
    pub interval: Option<String>,
}

pub async fn get_klines(
    Path(symbol): Path<String>,
    axum::extract::Query(KlineQuery { interval }): axum::extract::Query<KlineQuery>,
) -> impl IntoResponse {
    // 简单起见，每次请求都创建一个 service。若后续需要，可放到 AppState 里复用。
    let svc = BinancePriceService::new();

    let interval = interval.unwrap_or_else(|| "1m".to_string());

    match svc.fetch_klines(&symbol, &interval, 200).await {
        Ok(klines) => (StatusCode::OK, Json(klines)).into_response(),
        Err(err) => {
            tracing::warn!(
                error = %err,
                %symbol,
                %interval,
                "failed to fetch klines from Binance"
            );
            (
                StatusCode::BAD_GATEWAY,
                "failed to fetch klines from Binance",
            )
                .into_response()
        }
    }
}
