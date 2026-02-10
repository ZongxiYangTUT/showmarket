pub mod handlers;
pub mod models;
pub mod services;
pub mod state;

use axum::{
    Router,
    routing::{get, get_service},
};
use state::AppState;
use tower_http::services::ServeDir;

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/", get(handlers::page::index))
        .route("/ws/prices", get(handlers::ws::ws_prices))
        .route("/api/klines/{symbol}", get(handlers::klines::get_klines))
        .nest_service("/static", get_service(ServeDir::new("static")))
        .with_state(state)
}
