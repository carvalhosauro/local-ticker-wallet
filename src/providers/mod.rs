pub mod brapi;
pub mod yahoo;

use crate::core::types::{Asset, AssetId, Candle, Dividend, Quote};

/// Market-data provider. Uses native async-in-trait (Rust 1.75 RPITIT); every
/// returned future is `+ Send` so providers can be driven on a multi-threaded
/// runtime. Concrete impls live in `yahoo`/`brapi` (real HTTP in Tasks 10-11).
pub trait Provider: Send + Sync {
    fn name(&self) -> &'static str;
    fn quote(&self, a: &AssetId) -> impl std::future::Future<Output = anyhow::Result<Quote>> + Send;
    fn history(
        &self,
        a: &AssetId,
    ) -> impl std::future::Future<Output = anyhow::Result<Vec<Candle>>> + Send;
    fn dividends(
        &self,
        a: &AssetId,
    ) -> impl std::future::Future<Output = anyhow::Result<Vec<Dividend>>> + Send;
    fn search(
        &self,
        q: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<Vec<Asset>>> + Send;
}

/// Concrete provider set. Native `async fn` in traits is not object-safe, so the
/// fallback `Chain` stores this enum rather than `Box<dyn Provider>` and
/// match-delegates to the real `Provider` impls.
pub enum AnyProvider {
    Yahoo(yahoo::YahooProvider),
    Brapi(brapi::BrapiProvider),
    #[cfg(test)]
    TestFailing,
    #[cfg(test)]
    TestWorking,
}

impl AnyProvider {
    pub fn name(&self) -> &'static str {
        match self {
            AnyProvider::Yahoo(_) => "yahoo",
            AnyProvider::Brapi(_) => "brapi",
            #[cfg(test)]
            AnyProvider::TestFailing => "failing",
            #[cfg(test)]
            AnyProvider::TestWorking => "working",
        }
    }
    pub async fn quote(&self, a: &AssetId) -> anyhow::Result<Quote> {
        match self {
            AnyProvider::Yahoo(p) => p.quote(a).await,
            AnyProvider::Brapi(p) => p.quote(a).await,
            #[cfg(test)]
            AnyProvider::TestFailing => anyhow::bail!("down"),
            #[cfg(test)]
            AnyProvider::TestWorking => Ok(Quote {
                asset: a.clone(),
                price: rust_decimal_macros::dec!(9),
                prev_close: rust_decimal_macros::dec!(9),
                day_high: rust_decimal_macros::dec!(9),
                day_low: rust_decimal_macros::dec!(9),
                currency: "BRL".into(),
                source: "working".into(),
                fetched_at: chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
            }),
        }
    }
    pub async fn history(&self, a: &AssetId) -> anyhow::Result<Vec<Candle>> {
        match self {
            AnyProvider::Yahoo(p) => p.history(a).await,
            AnyProvider::Brapi(p) => p.history(a).await,
            #[cfg(test)]
            AnyProvider::TestFailing => anyhow::bail!("down"),
            #[cfg(test)]
            AnyProvider::TestWorking => Ok(vec![]),
        }
    }
    pub async fn dividends(&self, a: &AssetId) -> anyhow::Result<Vec<Dividend>> {
        match self {
            AnyProvider::Yahoo(p) => p.dividends(a).await,
            AnyProvider::Brapi(p) => p.dividends(a).await,
            #[cfg(test)]
            AnyProvider::TestFailing => anyhow::bail!("down"),
            #[cfg(test)]
            AnyProvider::TestWorking => Ok(vec![]),
        }
    }
    pub async fn search(&self, q: &str) -> anyhow::Result<Vec<Asset>> {
        match self {
            AnyProvider::Yahoo(p) => p.search(q).await,
            AnyProvider::Brapi(p) => p.search(q).await,
            #[cfg(test)]
            AnyProvider::TestFailing => anyhow::bail!("down"),
            #[cfg(test)]
            AnyProvider::TestWorking => Ok(vec![]),
        }
    }
}

/// Ordered fallback chain. Each method tries providers in order, returning the
/// first `Ok`; if all fail it returns the last `Err`.
pub struct Chain {
    pub providers: Vec<AnyProvider>,
}

impl Chain {
    pub fn new(providers: Vec<AnyProvider>) -> Self {
        Self { providers }
    }

    pub async fn quote(&self, a: &AssetId) -> anyhow::Result<Quote> {
        let mut last = anyhow::anyhow!("no providers configured");
        for p in &self.providers {
            match p.quote(a).await {
                Ok(v) => return Ok(v),
                Err(e) => last = e.context(format!("provider {} failed", p.name())),
            }
        }
        Err(last)
    }
    pub async fn history(&self, a: &AssetId) -> anyhow::Result<Vec<Candle>> {
        let mut last = anyhow::anyhow!("no providers configured");
        for p in &self.providers {
            match p.history(a).await {
                Ok(v) => return Ok(v),
                Err(e) => last = e.context(format!("provider {} failed", p.name())),
            }
        }
        Err(last)
    }
    pub async fn dividends(&self, a: &AssetId) -> anyhow::Result<Vec<Dividend>> {
        let mut last = anyhow::anyhow!("no providers configured");
        for p in &self.providers {
            match p.dividends(a).await {
                Ok(v) => return Ok(v),
                Err(e) => last = e.context(format!("provider {} failed", p.name())),
            }
        }
        Err(last)
    }
    pub async fn search(&self, q: &str) -> anyhow::Result<Vec<Asset>> {
        let mut last = anyhow::anyhow!("no providers configured");
        for p in &self.providers {
            match p.search(q).await {
                Ok(v) => return Ok(v),
                Err(e) => last = e.context(format!("provider {} failed", p.name())),
            }
        }
        Err(last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn chain_falls_through_to_working_provider() {
        let chain = Chain::new(vec![AnyProvider::TestFailing, AnyProvider::TestWorking]);
        let q = chain.quote(&AssetId::b3("PETR4")).await.unwrap();
        assert_eq!(q.source, "working");
    }
}
