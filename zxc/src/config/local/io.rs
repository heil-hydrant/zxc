use std::fs::{self, read_to_string};

use super::proxy::ProxyArgs;
use crate::config::CONFIG_FILE_NAME;
use crate::config::error::ConfigError;

/* Steps:
 *      1. If !should_attach i.e. new_session, write new config
 *
 *      2. If should_attach i.e. attach to existing session,
 *          a. Try reading old config, if ok, combine with new config and
 *          sanitize
 *          b. If not ok or no old config, write new config
 *
 *      3. Convert new_config to toml and write to session_config
 */

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

/* Error:
 *      ConfigError::FileRead [2]
 *      ConfigError::ParseConfig [3]
 */

pub fn parse_local_config() -> Result<ProxyArgs, ConfigError> {
    let contents = read_to_string(CONFIG_FILE_NAME)?;
    Ok(toml::from_str::<ProxyArgs>(&contents)?)
}
