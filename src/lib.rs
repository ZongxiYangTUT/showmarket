pub mod handlers;
pub mod models;
pub mod services;
pub mod state;

use axum::{Router, routing::get};
use state::AppState;

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health::get_health))
        .route("/price/btc", get(handlers::price::get_btc_price))
        .route("/ws/prices", get(handlers::ws::ws_prices))
        .with_state(state)
}
