use crate::models::kline::Kline;
use crate::models::price::PriceUpdate;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 模拟的 A 股行情服务。
///
/// 目前不访问任何外部接口，只在内存中生成 K 线和实时价格，
/// 方便在无法连外网的环境下调试 UI 和交互。
#[derive(Clone)]
pub struct AshareService;

impl AshareService {
    pub fn new() -> Self {
        Self
    }

    /// 生成模拟 K 线数据，按给定周期和数量回溯。
    pub async fn fetch_klines(
        &self,
        symbol: &str,
        interval: &str,
        limit: u16,
    ) -> anyhow::Result<Vec<Kline>> {
        let interval_ms = interval_to_ms(interval);
        let now_ms = now_ms();
        let limit = limit.min(500) as usize;

        // 简单随机游走生成价格
        let mut klines = Vec::with_capacity(limit);
        let mut price = base_price_for(symbol);

        for i in 0..limit {
            let idx_from_end = (limit - 1 - i) as i64;
            let open_time = now_ms - interval_ms * (idx_from_end + 1);

            // 生成四个价格
            let step = (symbol_hash(symbol) as f64 % 5.0 + 1.0) * 0.1;
            let noise = ((open_time / 1000) as f64).sin() * step;
            let open = price + noise;
            let close = open + (i as f64).sin() * step * 0.2;
            let high = open.max(close) + step * 0.5;
            let low = open.min(close) - step * 0.5;
            let volume = 1_000.0 + (i as f64 * 37.0) % 10_000.0;

            klines.push(Kline {
                open_time,
                open,
                high,
                low,
                close,
                volume,
            });

            price = close;
        }

        Ok(klines)
    }

    /// 生成某个股票的模拟最新价（用于本地测试，真实环境请替换为实际行情源）。
    pub fn next_mock_price(&self, symbol: &str, prev: Option<f64>) -> PriceUpdate {
        let base = prev.unwrap_or_else(|| base_price_for(symbol));
        let t = now_ms() as f64 / 1000.0;
        let step = (symbol_hash(symbol) as f64 % 3.0 + 1.0) * 0.2;
        let delta = (t / 5.0).sin() * step;
        let price = (base + delta).max(0.01);

        PriceUpdate {
            symbol: symbol.to_string(),
            price,
            ts_ms: now_ms(),
        }
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_millis() as i64
}

fn interval_to_ms(interval: &str) -> i64 {
    match interval {
        "1m" => 60_000,
        "5m" => 5 * 60_000,
        "15m" => 15 * 60_000,
        "30m" => 30 * 60_000,
        "1h" => 60 * 60_000,
        "4h" => 4 * 60_60_000 / 60, // avoid overflow
        "1d" => 24 * 60 * 60_000,
        "1w" => 7 * 24 * 60 * 60_000,
        _ => 60_000,
    }
}

fn base_price_for(symbol: &str) -> f64 {
    match symbol {
        "600000.SH" => 10.0,
        "000001.SZ" => 12.0,
        "300750.SZ" => 180.0,
        _ => 20.0 + (symbol_hash(symbol) % 100) as f64,
    }
}

fn symbol_hash(symbol: &str) -> u64 {
    let mut h = 0u64;
    for b in symbol.bytes() {
        h = h.wrapping_mul(31).wrapping_add(b as u64);
    }
    h
}
