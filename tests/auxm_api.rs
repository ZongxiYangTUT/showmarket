use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn health_ok() {
    let app = showmarket::app(showmarket::state::AppState::new());
    let res = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn price_unavailable_when_not_ready() {
    let app = showmarket::app(showmarket::state::AppState::new());
    let res = app
        .oneshot(
            Request::builder()
                .uri("/price/btc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn price_returns_latest_when_set() {
    let state = showmarket::state::AppState::new();
    state
        .set_latest(showmarket::models::price::PriceUpdate {
            symbol: "BTCUSDT".to_string(),
            price: 123.45,
            ts_ms: 1,
        })
        .await;

    let app = showmarket::app(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/price/btc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
