use std::env::{self, VarError};
use std::fs::read_to_string;
use std::path::PathBuf;

use crate::config::*;

// $HOME/.config/zxc
pub fn global_config_path() -> Result<PathBuf, VarError> {
    let home_dir = env::var("HOME")?;
    let mut path = PathBuf::from(home_dir);
    path.push(".config/zxc");
    Ok(path)
}

/// Parses the global config and sanitizes it.
///
/// # Errors
///
/// Returns [`ConfigError`] when `HOME` is unset, the file cannot be read, or
/// the TOML is invalid.
pub fn parse_global_config() -> Result<Option<GlobalConfig>, ConfigError> {
    let mut path = global_config_path()?;
    path.push(CONFIG_FILE_NAME);
    let contents = read_to_string(&path)?;
    let config = toml::from_str::<GlobalConfig>(&contents)?;
    Ok(config.sanitize())
}
