use chrono::{NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId {
    pub symbol: String,
    pub exchange: String,
}

impl AssetId {
    pub fn b3(symbol: &str) -> Self {
        Self { symbol: symbol.to_uppercase(), exchange: "BVMF".to_string() }
    }
    /// Yahoo ticker for B3 symbols, e.g. PETR4 -> PETR4.SA
    pub fn yahoo_ticker(&self) -> String {
        if self.exchange == "BVMF" { format!("{}.SA", self.symbol) } else { self.symbol.clone() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Side { Buy, Sell }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trade {
    pub id: i64,
    pub asset: AssetId,
    pub side: Side,
    pub quantity: Decimal,
    pub price: Decimal,
    pub fees: Decimal,
    pub executed_at: NaiveDate,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quote {
    pub asset: AssetId,
    pub price: Decimal,
    pub prev_close: Decimal,
    pub day_high: Decimal,
    pub day_low: Decimal,
    pub currency: String,
    pub source: String,
    pub fetched_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candle {
    pub date: NaiveDate,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dividend {
    pub asset: AssetId,
    pub ex_date: NaiveDate,
    pub pay_date: Option<NaiveDate>,
    pub amount_per_share: Decimal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    pub id: AssetId,
    pub name: String,
    pub kind: String,
    pub currency: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn b3_asset_builds_yahoo_ticker() {
        let a = AssetId::b3("petr4");
        assert_eq!(a.symbol, "PETR4");
        assert_eq!(a.exchange, "BVMF");
        assert_eq!(a.yahoo_ticker(), "PETR4.SA");
    }

    #[test]
    fn side_serializes_uppercase() {
        assert_eq!(serde_json::to_string(&Side::Buy).unwrap(), "\"BUY\"");
    }
}
