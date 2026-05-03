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
