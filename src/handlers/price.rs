use crate::state::AppState;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};

pub async fn get_btc_price(State(state): State<AppState>) -> impl IntoResponse {
    match state.latest().await {
        Some(update) => (StatusCode::OK, Json(update)).into_response(),
        None => (StatusCode::SERVICE_UNAVAILABLE, "price not ready").into_response(),
    }
}
