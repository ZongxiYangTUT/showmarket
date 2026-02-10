use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PriceUpdate {
    pub symbol: String,
    pub price: f64,
    /// Unix timestamp (ms)
    pub ts_ms: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BinanceTickerPrice {
    pub symbol: String,
    /// Binance returns price as string
    pub price: String,
}
