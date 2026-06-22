use crate::core::types::{Asset, AssetId, Candle, Dividend, Quote};
use rust_decimal::Decimal;
use std::str::FromStr;

use super::Provider;

/// brapi.dev provider. Parses `/quote/{ticker}?range=1y&interval=1d&dividends=true`
/// and `/available?search=`. Token (optional) appended as `&token=` query param.
pub struct BrapiProvider {
    pub client: reqwest::Client,
    pub base_url: String,
    pub token: Option<String>,
}

impl BrapiProvider {
    pub fn new(base_url: String, token: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
            token,
        }
    }

    pub fn default_base(token: Option<String>) -> Self {
        Self::new("https://brapi.dev/api".into(), token)
    }

    async fn fetch(&self, a: &AssetId) -> anyhow::Result<serde_json::Value> {
        let mut url = format!(
            "{}/quote/{}?range=1y&interval=1d&dividends=true",
            self.base_url, a.symbol
        );
        if let Some(t) = &self.token {
            url.push_str(&format!("&token={t}"));
        }
        Ok(self.client.get(url).send().await?.error_for_status()?.json().await?)
    }
}

/// Parse a JSON number to `Decimal` without going through f64.
/// `serde_json::Number::to_string()` gives the original decimal representation,
/// which is then parsed losslessly via `Decimal::from_str`.
fn decimal_from_json(v: &serde_json::Value) -> Option<Decimal> {
    v.as_number()?.to_string().as_str().pipe(Decimal::from_str).ok()
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

impl Provider for BrapiProvider {
    fn name(&self) -> &'static str {
        "brapi"
    }

    async fn quote(&self, a: &AssetId) -> anyhow::Result<Quote> {
        let v = self.fetch(a).await?;
        let r = &v["results"][0];
        Ok(Quote {
            asset: a.clone(),
            price: decimal_from_json(&r["regularMarketPrice"])
                .ok_or_else(|| anyhow::anyhow!("no price"))?,
            prev_close: decimal_from_json(&r["regularMarketPreviousClose"]).unwrap_or_default(),
            day_high: decimal_from_json(&r["regularMarketDayHigh"]).unwrap_or_default(),
            day_low: decimal_from_json(&r["regularMarketDayLow"]).unwrap_or_default(),
            currency: r["currency"].as_str().unwrap_or("BRL").to_string(),
            source: "brapi".into(),
            fetched_at: chrono::Utc::now().naive_utc(),
        })
    }

    async fn history(&self, a: &AssetId) -> anyhow::Result<Vec<Candle>> {
        let v = self.fetch(a).await?;
        let mut out = Vec::new();
        if let Some(arr) = v["results"][0]["historicalDataPrice"].as_array() {
            for h in arr {
                let secs = h["date"].as_i64().unwrap_or(0);
                let Some(date) =
                    chrono::DateTime::from_timestamp(secs, 0).map(|dt| dt.naive_utc().date())
                else {
                    continue;
                };
                let (Some(o), Some(hi), Some(l), Some(c)) = (
                    decimal_from_json(&h["open"]),
                    decimal_from_json(&h["high"]),
                    decimal_from_json(&h["low"]),
                    decimal_from_json(&h["close"]),
                ) else {
                    continue;
                };
                out.push(Candle {
                    date,
                    open: o,
                    high: hi,
                    low: l,
                    close: c,
                    volume: h["volume"].as_i64().unwrap_or(0),
                });
            }
        }
        out.sort_by_key(|c| c.date);
        Ok(out)
    }

    async fn dividends(&self, a: &AssetId) -> anyhow::Result<Vec<Dividend>> {
        let v = self.fetch(a).await?;
        let mut out = Vec::new();
        if let Some(arr) = v["results"][0]["dividendsData"]["cashDividends"].as_array() {
            for dvd in arr {
                let Some(amt) = decimal_from_json(&dvd["rate"]) else { continue };
                let ex_date = dvd["lastDatePrior"]
                    .as_str()
                    .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
                let pay_date = dvd["paymentDate"]
                    .as_str()
                    .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
                let Some(ex_date) = ex_date else { continue };
                out.push(Dividend {
                    asset: a.clone(),
                    ex_date,
                    pay_date,
                    amount_per_share: amt,
                });
            }
        }
        out.sort_by_key(|d| d.ex_date);
        Ok(out)
    }

    async fn search(&self, query: &str) -> anyhow::Result<Vec<Asset>> {
        let mut url = format!("{}/available?search={}", self.base_url, query);
        if let Some(t) = &self.token {
            url.push_str(&format!("&token={t}"));
        }
        let v: serde_json::Value =
            self.client.get(url).send().await?.error_for_status()?.json().await?;
        let mut out = Vec::new();
        if let Some(arr) = v["stocks"].as_array() {
            for s in arr {
                if let Some(sym) = s.as_str() {
                    out.push(Asset {
                        id: AssetId::b3(sym),
                        name: sym.to_string(),
                        kind: "EQUITY".into(),
                        currency: "BRL".into(),
                    });
                }
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
    async fn parses_brapi_quote_and_history() {
        let server = MockServer::start().await;
        let body = include_str!("../../tests/fixtures/brapi_quote_petr4.json");
        Mock::given(method("GET"))
            .and(path_regex(r"^/quote/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(&server)
            .await;

        let p = BrapiProvider::new(server.uri(), None);
        let q = p.quote(&AssetId::b3("PETR4")).await.unwrap();
        assert_eq!(q.price, dec!(38.50));
        assert_eq!(q.source, "brapi");
        let c = p.history(&AssetId::b3("PETR4")).await.unwrap();
        assert_eq!(c.len(), 2);
        let d = p.dividends(&AssetId::b3("PETR4")).await.unwrap();
        assert_eq!(d[0].amount_per_share, dec!(0.55));
    }
}
