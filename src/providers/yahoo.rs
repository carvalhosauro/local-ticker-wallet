use crate::core::types::{Asset, AssetId, Candle, Dividend, Quote};

use super::Provider;

/// Yahoo Finance provider. Parses the `chart` endpoint for quote/history/dividends;
/// `search` hits `/v1/finance/search`. `base_url` is injectable for wiremock tests.
pub struct YahooProvider {
    pub client: reqwest::Client,
    pub base_url: String,
}

impl YahooProvider {
    pub fn new(base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 local-ticker-wallet")
            .timeout(std::time::Duration::from_secs(10))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("reqwest client");
        Self { client, base_url }
    }

    pub fn default_base() -> Self {
        Self::new("https://query1.finance.yahoo.com".to_string())
    }

    async fn fetch_chart(&self, a: &AssetId) -> anyhow::Result<serde_json::Value> {
        let url = format!(
            "{}/v8/finance/chart/{}?range=1y&interval=1d&events=div",
            self.base_url,
            a.yahoo_ticker()
        );
        let v: serde_json::Value =
            self.client.get(url).send().await?.error_for_status()?.json().await?;
        Ok(v)
    }
}

/// Parse a JSON number to `Decimal` without going through f64.
/// `serde_json::Number::to_string()` gives the original decimal representation,
/// which is then parsed losslessly via `Decimal::from_str`.
fn decimal_from_json(v: &serde_json::Value) -> Option<rust_decimal::Decimal> {
    use std::str::FromStr;
    v.as_number()?.to_string().as_str().pipe(rust_decimal::Decimal::from_str).ok()
}

// A tiny helper so the `.pipe()` call above compiles without an external crate.
trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R;
}
impl<T: Sized> Pipe for T {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
    {
        f(self)
    }
}

impl Provider for YahooProvider {
    fn name(&self) -> &'static str {
        "yahoo"
    }

    async fn quote(&self, a: &AssetId) -> anyhow::Result<Quote> {
        let v = self.fetch_chart(a).await?;
        let meta = &v["chart"]["result"][0]["meta"];
        Ok(Quote {
            asset: a.clone(),
            price: decimal_from_json(&meta["regularMarketPrice"])
                .ok_or_else(|| anyhow::anyhow!("no price"))?,
            prev_close: decimal_from_json(&meta["chartPreviousClose"]).unwrap_or_default(),
            day_high: decimal_from_json(&meta["regularMarketDayHigh"]).unwrap_or_default(),
            day_low: decimal_from_json(&meta["regularMarketDayLow"]).unwrap_or_default(),
            currency: meta["currency"].as_str().unwrap_or("BRL").to_string(),
            source: "yahoo".into(),
            fetched_at: chrono::Utc::now().naive_utc(),
        })
    }

    async fn history(&self, a: &AssetId) -> anyhow::Result<Vec<Candle>> {
        let v = self.fetch_chart(a).await?;
        let result = &v["chart"]["result"][0];
        let ts = result["timestamp"].as_array().cloned().unwrap_or_default();
        let q = &result["indicators"]["quote"][0];
        let mut out = Vec::new();
        for (i, t) in ts.iter().enumerate() {
            let secs = t.as_i64().unwrap_or(0);
            let date = chrono::DateTime::from_timestamp(secs, 0)
                .map(|dt| dt.naive_utc().date());
            let (Some(date), Some(o), Some(h), Some(l), Some(c)) = (
                date,
                decimal_from_json(&q["open"][i]),
                decimal_from_json(&q["high"][i]),
                decimal_from_json(&q["low"][i]),
                decimal_from_json(&q["close"][i]),
            ) else {
                continue;
            };
            out.push(Candle {
                date,
                open: o,
                high: h,
                low: l,
                close: c,
                volume: q["volume"][i].as_i64().unwrap_or(0),
            });
        }
        Ok(out)
    }

    async fn dividends(&self, a: &AssetId) -> anyhow::Result<Vec<Dividend>> {
        let v = self.fetch_chart(a).await?;
        let divs = &v["chart"]["result"][0]["events"]["dividends"];
        let mut out = Vec::new();
        if let Some(map) = divs.as_object() {
            for (_, dv) in map {
                let secs = dv["date"].as_i64().unwrap_or(0);
                let Some(ex_date) =
                    chrono::DateTime::from_timestamp(secs, 0).map(|dt| dt.naive_utc().date())
                else {
                    continue;
                };
                let Some(amt) = decimal_from_json(&dv["amount"]) else { continue };
                out.push(Dividend {
                    asset: a.clone(),
                    ex_date,
                    pay_date: None,
                    amount_per_share: amt,
                });
            }
        }
        out.sort_by_key(|d| d.ex_date);
        Ok(out)
    }

    async fn search(&self, query: &str) -> anyhow::Result<Vec<Asset>> {
        let encoded: String = query
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_string()
                } else {
                    format!("%{:02X}", c as u32)
                }
            })
            .collect();
        let url = format!("{}/v1/finance/search?q={}", self.base_url, encoded);
        let v: serde_json::Value =
            self.client.get(url).send().await?.error_for_status()?.json().await?;
        let mut out = Vec::new();
        if let Some(items) = v["quotes"].as_array() {
            for it in items {
                let sym = it["symbol"].as_str().unwrap_or("");
                let symbol = sym.strip_suffix(".SA").unwrap_or(sym).to_string();
                out.push(Asset {
                    id: AssetId { symbol, exchange: "BVMF".into() },
                    name: it["shortname"].as_str().unwrap_or("").to_string(),
                    kind: it["quoteType"].as_str().unwrap_or("EQUITY").to_string(),
                    currency: "BRL".into(),
                });
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use wiremock::matchers::{method, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn parses_quote_from_chart() {
        let server = MockServer::start().await;
        let body = include_str!("../../tests/fixtures/yahoo_chart_petr4.json");
        Mock::given(method("GET"))
            .and(path_regex(r"^/v8/finance/chart/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(&server)
            .await;

        let p = YahooProvider::new(server.uri());
        let q = p.quote(&AssetId::b3("PETR4")).await.unwrap();
        assert_eq!(q.price, dec!(38.50));
        assert_eq!(q.prev_close, dec!(37.90));
        assert_eq!(q.currency, "BRL");
        assert_eq!(q.source, "yahoo");

        let candles = p.history(&AssetId::b3("PETR4")).await.unwrap();
        assert_eq!(candles.len(), 2);
        assert_eq!(candles[1].close, dec!(38.5));

        let divs = p.dividends(&AssetId::b3("PETR4")).await.unwrap();
        assert_eq!(divs.len(), 1);
        assert_eq!(divs[0].amount_per_share, dec!(0.55));
    }
}
