use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::CliResult;
use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_query_files_preview_chars")]
    pub query_files_preview_chars: usize,
    #[serde(default = "default_query_text_preview_chars")]
    pub query_text_preview_chars: usize,
    #[serde(default)]
    pub graph_ranking: GraphRankingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphRankingConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_max_related_suggestions")]
    pub max_related_suggestions: usize,
    #[serde(default = "default_max_file_fanout")]
    pub max_file_fanout: usize,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    #[serde(default = "default_tolerance")]
    pub tolerance: f64,
    #[serde(default = "default_damping")]
    pub damping: f64,
    #[serde(default = "default_recency_half_life_days")]
    pub recency_half_life_days: f64,
}

impl Default for GraphRankingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_related_suggestions: 20,
            max_file_fanout: 100,
            max_iterations: 80,
            tolerance: 1e-6,
            damping: 0.85,
            recency_half_life_days: 365.0,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_max_related_suggestions() -> usize {
    20
}

fn default_max_file_fanout() -> usize {
    100
}

fn default_max_iterations() -> usize {
    80
}

fn default_tolerance() -> f64 {
    1e-6
}

fn default_damping() -> f64 {
    0.85
}

fn default_recency_half_life_days() -> f64 {
    365.0
}

fn default_query_files_preview_chars() -> usize {
    2_000
}

fn default_query_text_preview_chars() -> usize {
    200
}

impl Default for Config {
    fn default() -> Self {
        Self {
            query_files_preview_chars: default_query_files_preview_chars(),
            query_text_preview_chars: default_query_text_preview_chars(),
            graph_ranking: GraphRankingConfig::default(),
        }
    }
}

pub fn load_or_create(root: &Path) -> CliResult<Config> {
    let config_path = paths::config_path(root);
    if config_path.exists() {
        let raw = std::fs::read_to_string(&config_path).map_err(|err| {
            crate::error::CliError::Storage(format!(
                "Unable to read config '{}': {err}",
                config_path.display()
            ))
        })?;
        serde_json::from_str(&raw).map_err(|err| {
            crate::error::CliError::Validation(format!(
                "Invalid config '{}': {err}",
                config_path.display()
            ))
        })
    } else {
        let config = Config::default();
        let json = serde_json::to_string_pretty(&config).map_err(|err| {
            crate::error::CliError::Storage(format!("Unable to serialize default config: {err}"))
        })?;
        std::fs::write(&config_path, json).map_err(|err| {
            crate::error::CliError::Storage(format!(
                "Unable to write config '{}': {err}",
                config_path.display()
            ))
        })?;
        Ok(config)
    }
}
