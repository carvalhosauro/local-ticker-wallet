use crate::core::types::{Asset, AssetId, Candle, Dividend, Quote};

use super::Provider;

/// brapi.dev provider. Real HTTP implementation lands in Task 11; this is a
/// compiling stub so `AnyProvider::Brapi` delegation type-checks.
pub struct BrapiProvider {
    pub client: reqwest::Client,
    pub token: Option<String>,
}

impl Provider for BrapiProvider {
    fn name(&self) -> &'static str {
        "brapi"
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
