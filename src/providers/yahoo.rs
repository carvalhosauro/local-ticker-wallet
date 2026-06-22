use crate::core::types::{Asset, AssetId, Candle, Dividend, Quote};

use super::Provider;

/// Yahoo Finance provider. Real HTTP implementation lands in Task 10; this is a
/// compiling stub so `AnyProvider::Yahoo` delegation type-checks.
pub struct YahooProvider {
    pub client: reqwest::Client,
}

impl Provider for YahooProvider {
    fn name(&self) -> &'static str {
        "yahoo"
    }
    async fn quote(&self, _: &AssetId) -> anyhow::Result<Quote> {
        anyhow::bail!("not implemented")
    }
    async fn history(&self, _: &AssetId) -> anyhow::Result<Vec<Candle>> {
        anyhow::bail!("not implemented")
    }
    async fn dividends(&self, _: &AssetId) -> anyhow::Result<Vec<Dividend>> {
        anyhow::bail!("not implemented")
    }
    async fn search(&self, _: &str) -> anyhow::Result<Vec<Asset>> {
        anyhow::bail!("not implemented")
    }
}
