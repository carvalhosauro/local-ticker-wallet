use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreWeights {
    pub proximity_low: u32,
    pub below_sma: u32,
    pub drawdown: u32,
    pub dividend_yield: u32,
    pub cost_vs_trend: u32,
}

impl Default for ScoreWeights {
    fn default() -> Self {
        Self {
            proximity_low: 25,
            below_sma: 20,
            drawdown: 15,
            dividend_yield: 20,
            cost_vs_trend: 20,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub brapi_token: Option<String>,
    pub poll_interval_secs: u64,
    pub score_weights: ScoreWeights,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            brapi_token: None,
            poll_interval_secs: 60,
            score_weights: ScoreWeights::default(),
        }
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Config> {
        let path = crate::paths::config_file();
        if !path.exists() {
            let cfg = Config::default();
            std::fs::write(&path, serde_json::to_string_pretty(&cfg)?)?;
            return Ok(cfg);
        }
        let text = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&text)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_roundtrips_through_json() {
        let cfg = Config::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
        assert_eq!(back.poll_interval_secs, 60);
        assert_eq!(back.score_weights.proximity_low, 25);
    }

    #[test]
    fn partial_json_fills_defaults() {
        let back: Config = serde_json::from_str("{\"poll_interval_secs\": 30}").unwrap();
        assert_eq!(back.poll_interval_secs, 30);
        assert_eq!(back.score_weights.dividend_yield, 20);
    }
}
