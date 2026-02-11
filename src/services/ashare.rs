use crate::models::kline::Kline;
use crate::models::price::PriceUpdate;
use anyhow::{Context, anyhow};
use chrono::{Local, NaiveDate, NaiveDateTime, TimeZone};
use serde::Deserialize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A 股行情服务。
///
/// - 历史 K 线：使用东方财富 push2his K 线接口
/// - 实时价格：使用东方财富 push2 实时行情接口
///
/// 后续如果你有自己的行情中台，只需在这里替换调用即可。
#[derive(Clone)]
pub struct AshareService {
    client: reqwest::Client,
}

impl AshareService {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .user_agent("showmarket-ashare/0.1")
            .build()
            .expect("failed to build reqwest client");
        Self { client }
    }

    /// 获取真实 A 股 K 线数据。
    ///
    /// `symbol` 形如 "600000.SH" / "000001.SZ" / "300750.SZ"
    /// `interval` 映射为东方财富的 klt 参数：
    ///   1m -> 101, 5m -> 102, 15m -> 103, 30m -> 104,
    ///   1h -> 105, 1d -> 106, 1w -> 107, 1M -> 108
    pub async fn fetch_klines(
        &self,
        symbol: &str,
        interval: &str,
        limit: u16,
    ) -> anyhow::Result<Vec<Kline>> {
        let secid = to_secid(symbol).context("unsupported symbol")?;
        let klt = to_klt(interval)?;
        let limit = limit.min(500);

        let url = format!(
            "https://push2his.eastmoney.com/api/qt/stock/kline/get\
             ?secid={secid}&klt={klt}&fqt=1&end=20500101&lmt={limit}\
             &fields1=f1,f2,f3,f4,f5&fields2=f51,f52,f53,f54,f55,f56,f57,f58"
        );

        let resp = self.client.get(url).send().await?.error_for_status()?;

        let body = resp.text().await?;
        let em: EmKlineResp = serde_json::from_str(&body)
            .with_context(|| format!("parse kline response failed: {body}"))?;

        let data = em.data.ok_or_else(|| anyhow!("empty kline data"))?;
        let mut out = Vec::with_capacity(data.klines.len());

        for s in data.klines {
            // "2024-02-10 09:30,open,close,high,low,volume,amount,amplitude"
            let parts: Vec<&str> = s.split(',').collect();
            if parts.len() < 6 {
                continue;
            }
            let ts_ms = parse_em_time(parts[0]);
            let open = parts[1].parse::<f64>().unwrap_or(0.0);
            let close = parts[2].parse::<f64>().unwrap_or(0.0);
            let high = parts[3].parse::<f64>().unwrap_or(0.0);
            let low = parts[4].parse::<f64>().unwrap_or(0.0);
            let volume = parts[5].parse::<f64>().unwrap_or(0.0);

            out.push(Kline {
                open_time: ts_ms,
                open,
                high,
                low,
                close,
                volume,
            });
        }

        Ok(out)
    }

    /// 获取某支股票的真实最新价（东方财富推送接口）。
    pub async fn fetch_realtime_quote(&self, symbol: &str) -> anyhow::Result<PriceUpdate> {
        let secid = to_secid(symbol).context("unsupported symbol")?;
        // 只取最新价 f43
        let url = format!(
            "https://push2.eastmoney.com/api/qt/stock/get\
             ?secid={secid}&fields=f43"
        );

        let resp = self.client.get(url).send().await?.error_for_status()?;

        let body = resp.text().await?;
        let em: EmQuoteResp =
            serde_json::from_str(&body).with_context(|| format!("parse quote failed: {body}"))?;

        let data = em.data.ok_or_else(|| anyhow!("empty quote data"))?;
        // 东方财富推送的 f43 通常是价格 * 100，单位为“分”
        let raw = data.f43;
        let price = raw / 100.0;

        Ok(PriceUpdate {
            symbol: symbol.to_string(),
            price,
            ts_ms: now_ms(),
        })
    }
}

#[derive(Debug, Deserialize)]
struct EmKlineResp {
    data: Option<EmKlineData>,
}

#[derive(Debug, Deserialize)]
struct EmKlineData {
    klines: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct EmQuoteResp {
    data: Option<EmQuoteData>,
}

#[derive(Debug, Deserialize)]
struct EmQuoteData {
    #[serde(rename = "f43")]
    f43: f64,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_millis() as i64
}

fn to_secid(symbol: &str) -> Option<String> {
    if let Some(code) = symbol.strip_suffix(".SH") {
        Some(format!("1.{}", code))
    } else if let Some(code) = symbol.strip_suffix(".SZ") {
        Some(format!("0.{}", code))
    } else {
        None
    }
}

fn to_klt(interval: &str) -> anyhow::Result<u32> {
    let v = match interval {
        "1m" => 101,
        "5m" => 102,
        "15m" => 103,
        "30m" => 104,
        "1h" => 105,
        "1d" => 106,
        "1w" => 107,
        "1M" => 108,
        other => return Err(anyhow!("unsupported interval: {other}")),
    };
    Ok(v)
}

fn parse_em_time(s: &str) -> i64 {
    // 支持 "YYYY-MM-DD HH:MM"（分时/分钟）和 "YYYY-MM-DD"（日/周/月）
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M") {
        let dt = Local
            .from_local_datetime(&ndt)
            .single()
            .unwrap_or_else(|| Local.timestamp_millis_opt(0).single().unwrap());
        dt.timestamp_millis()
    } else if let Ok(nd) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let ndt = nd
            .and_hms_opt(15, 0, 0)
            .unwrap_or_else(|| nd.and_hms_opt(0, 0, 0).unwrap());
        let dt = Local
            .from_local_datetime(&ndt)
            .single()
            .unwrap_or_else(|| Local.timestamp_millis_opt(0).single().unwrap());
        dt.timestamp_millis()
    } else {
        now_ms()
    }
}
