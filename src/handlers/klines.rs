use axum::{Json, extract::Path, http::StatusCode, response::IntoResponse};

use crate::services::ashare::AshareService;

#[derive(serde::Deserialize)]
pub struct KlineQuery {
    pub interval: Option<String>,
}

pub async fn get_klines(
    Path(symbol): Path<String>,
    axum::extract::Query(KlineQuery { interval }): axum::extract::Query<KlineQuery>,
) -> impl IntoResponse {
    // 简单起见，每次请求都创建一个 service。后面可放到 AppState 里复用或共享连接。
    let svc = AshareService::new();

    let interval = interval.unwrap_or_else(|| "1m".to_string());

    match svc.fetch_klines(&symbol, &interval, 200).await {
        Ok(klines) => (StatusCode::OK, Json(klines)).into_response(),
        Err(err) => {
            tracing::warn!(
                error = %err,
                %symbol,
                %interval,
                "failed to fetch klines from Ashare service"
            );
            (
                StatusCode::BAD_GATEWAY,
                "failed to fetch klines from Ashare service",
            )
                .into_response()
        }
    }
}
