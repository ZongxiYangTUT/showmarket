use crate::state::AppState;
use axum::extract::ws::{Message, WebSocket};
use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
};
use tokio::select;

pub async fn ws_prices(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    // Send latest immediately if we have it.
    if let Some(latest) = state.latest().await {
        if let Ok(txt) = serde_json::to_string(&latest) {
            let _ = socket.send(Message::Text(txt.into())).await;
        }
    }

    let mut rx = state.subscribe();

    loop {
        select! {
            // Broadcast -> client
            msg = rx.recv() => {
                match msg {
                    Ok(update) => {
                        let Ok(txt) = serde_json::to_string(&update) else { continue };
                        if socket.send(Message::Text(txt.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        // skip missed messages
                        continue;
                    }
                    Err(_) => break,
                }
            }
            // Client -> server (we just drain; allow close)
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => continue,
                    Some(Err(_)) => break,
                }
            }
        }
    }
}
