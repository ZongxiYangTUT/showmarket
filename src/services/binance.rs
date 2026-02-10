use crate::models::{
    kline::Kline,
    price::{BinanceTickerPrice, PriceUpdate},
};
use anyhow::Context;
use std::time::Duration;

#[derive(Clone)]
pub struct BinancePriceService {
    client: reqwest::Client,
    base_url: String,
}

impl BinancePriceService {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api.binance.com".to_string(),
        }
    }

    /// For tests / custom endpoints.
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    pub async fn fetch_btcusdt(&self) -> anyhow::Result<PriceUpdate> {
        let url = format!("{}/api/v3/ticker/price?symbol=BTCUSDT", self.base_url);
        let resp = self
            .client
            .get(url)
            .timeout(Duration::from_secs(5))
            .send()
            .await?
            .error_for_status()?;

        let ticker: BinanceTickerPrice = resp.json().await?;
        let price = ticker
            .price
            .parse::<f64>()
            .with_context(|| format!("invalid Binance price: {}", ticker.price))?;

        Ok(PriceUpdate {
            symbol: ticker.symbol,
            price,
            ts_ms: chrono_ms_now(),
        })
    }

    /// Fetch recent klines for a symbol and interval, e.g. ("BTCUSDT", "1m").
    pub async fn fetch_klines(
        &self,
        symbol: &str,
        interval: &str,
        limit: u16,
    ) -> anyhow::Result<Vec<Kline>> {
        let url = format!(
            "{}/api/v3/klines?symbol={symbol}&interval={interval}&limit={limit}",
            self.base_url
        );
        let resp = self
            .client
            .get(url)
            .timeout(Duration::from_secs(5))
            .send()
            .await?
            .error_for_status()?;

        // Each kline is an array; see Binance docs.
        let raw: Vec<Vec<serde_json::Value>> = resp.json().await?;
        let mut klines = Vec::with_capacity(raw.len());

        for entry in raw {
            if entry.len() < 6 {
                continue;
            }

            let open_time = entry[0].as_i64().unwrap_or(0);
            let open = entry[1]
                .as_str()
                .unwrap_or("0")
                .parse::<f64>()
                .unwrap_or(0.0);
            let high = entry[2]
                .as_str()
                .unwrap_or("0")
                .parse::<f64>()
                .unwrap_or(0.0);
            let low = entry[3]
                .as_str()
                .unwrap_or("0")
                .parse::<f64>()
                .unwrap_or(0.0);
            let close = entry[4]
                .as_str()
                .unwrap_or("0")
                .parse::<f64>()
                .unwrap_or(0.0);
            let volume = entry[5]
                .as_str()
                .unwrap_or("0")
                .parse::<f64>()
                .unwrap_or(0.0);

            klines.push(Kline {
                open_time,
                open,
                high,
                low,
                close,
                volume,
            });
        }

        Ok(klines)
    }
}

fn chrono_ms_now() -> i64 {
    // avoid extra chrono dependency
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    dur.as_millis() as i64
}
