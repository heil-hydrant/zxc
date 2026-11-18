use std::fs::{self, read_to_string};

use crate::config::*;

/// Writes the local config for the session.
///
/// # Steps
///
/// 1. If `should_attach` is false, i.e. a new session, write the new
///    config.
/// 2. If `should_attach` is true, i.e. attaching to an existing session:
///    - Try reading the old config; on success combine it with the new
///      config and sanitize the result.
///    - If reading fails or there is no old config, write the new config.
/// 3. Convert the resulting config to TOML and write it to the session
///    config file.
pub fn write_local_config(
    should_attach: bool,
    new_config: ProxyArgs,
) -> Option<ProxyArgs> {
    let args = if !should_attach {
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
///
/// # Errors
///
/// - `ConfigError::FileRead` [2]
/// - `ConfigError::ParseConfig` [3]
pub fn parse_local_config() -> Result<ProxyArgs, ConfigError> {
    let contents = read_to_string(CONFIG_FILE_NAME)?;
    Ok(toml::from_str::<ProxyArgs>(&contents)?)
}
