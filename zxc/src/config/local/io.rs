use std::fs::{self, read_to_string};

use crate::config::*;

/// Writes the local config for the session.
pub fn write_local_config(
    should_attach: bool,
    new_config: ProxyArgs,
) -> Option<ProxyArgs> {
    let args = if !should_attach {
        // new session
        Some(new_config)
    } else {
        match fs::read_to_string(CONFIG_FILE_NAME) {
            Ok(contents) => match toml::from_str::<ProxyArgs>(&contents) {
                Ok(old_config) => {
                    let combined = new_config + old_config;
                    combined.sanitize()
                }
                Err(e) => {
                    eprintln!("toml parse old config| {}", e);
                    Some(new_config)
                }
            },
            Err(e) => {
                eprintln!("read old config| {}", e);
                Some(new_config)
            }
        }
    };

    if let Some(parsed) = args {
        match toml::to_string(&parsed) {
            Ok(config) => {
                if !config.is_empty()
                    && let Err(e) = fs::write(CONFIG_FILE_NAME, config)
                {
                    eprintln!("write config| {}", e);
                }
            }
            Err(e) => {
                eprintln!("toml parse final config| {}", e);
            }
        }
        return Some(parsed);
    }
    args
}

/// Reads and parses the local config file.
pub fn parse_local_config() -> Result<ProxyArgs, ConfigError> {
    let contents = read_to_string(CONFIG_FILE_NAME)?;
    Ok(toml::from_str::<ProxyArgs>(&contents)?)
}
