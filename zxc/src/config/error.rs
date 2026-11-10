use std::env::VarError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    // ----- Local Config -----
    #[error("config read: {0}")]
    FileRead(#[from] std::io::Error),
    #[error("config parse: {0}")]
    ParseConfig(#[from] toml::de::Error),

    // ----- Global Config -----
    #[error("home directory lookup: {0}")]
    Env(#[from] VarError),
}
