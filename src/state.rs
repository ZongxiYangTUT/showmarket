use crate::models::price::PriceUpdate;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

#[derive(Clone)]
pub struct AppState {
    latest: Arc<RwLock<Option<PriceUpdate>>>,
    tx: broadcast::Sender<PriceUpdate>,
}

impl AppState {
    pub fn new() -> Self {
        // small buffer; slow clients may miss updates, which is fine for a ticker
        let (tx, _) = broadcast::channel(32);
        Self {
            latest: Arc::new(RwLock::new(None)),
            tx,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<PriceUpdate> {
        self.tx.subscribe()
    }

    pub async fn set_latest(&self, update: PriceUpdate) {
        *self.latest.write().await = Some(update.clone());
        // ignore lagging/no receivers
        let _ = self.tx.send(update);
    }

    pub async fn latest(&self) -> Option<PriceUpdate> {
        self.latest.read().await.clone()
    }
}
