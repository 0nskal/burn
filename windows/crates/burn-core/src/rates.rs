//! Port of `ModelRates.swift`: the LiteLLM price table, T3's overrides and aliases, pricing.

use crate::parsers::Speed;
use crate::paths::Paths;
use std::collections::HashMap;

/// Per-token prices of one speed tier.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TokenRates {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_creation: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelRate {
    pub standard: TokenRates,
    pub fast: Option<TokenRates>,
    pub ultrafast: Option<TokenRates>,
}

#[derive(Clone, Debug, Default)]
pub struct ModelRates {
    pub table: HashMap<String, ModelRate>,
    pub overrides: HashMap<String, TokenRates>,
    pub aliases: HashMap<String, String>,
    /// Identifies the table, overrides and aliases; a change means stored costs are stale.
    pub hash: String,
}

impl ModelRates {
    pub fn empty() -> ModelRates {
        ModelRates { hash: "empty".into(), ..Default::default() }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn cost(
        &self,
        model: &str,
        rate_model: Option<&str>,
        speed: Speed,
        uncached: i64,
        cached: i64,
        cache_creation: i64,
        output: i64,
        reported: Option<f64>,
    ) -> f64 {
        let _ = (model, rate_model, speed, uncached, cached, cache_creation, output, reported);
        todo!()
    }

    /// T3's rates copy, then fresh Burn cache, then network, then any stale copy, then nothing. Blocking.
    pub fn load(paths: &Paths) -> ModelRates {
        let _ = paths;
        todo!()
    }

    /// Rates exactly as T3 has them on disk, regardless of age (verification).
    pub fn load_t3_only(paths: &Paths) -> Option<ModelRates> {
        let _ = paths;
        todo!()
    }
}
