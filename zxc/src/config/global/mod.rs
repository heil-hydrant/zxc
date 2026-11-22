pub mod addons;

use std::collections::HashMap;
use std::env::{self, VarError};
use std::fs::read_to_string;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use mime_plz::ContentType;
use serde::Deserialize;

use crate::config::*;

#[cfg_attr(any(test, debug_assertions), derive(PartialEq))]
#[derive(Deserialize, Debug)]
pub struct GlobalConfig {
    pub excluded_domains: Option<Vec<String>>,
    pub excluded_content_types: Option<Vec<ContentType>>,
    pub excluded_extensions: Option<Vec<String>>,
    pub with_ws: Option<bool>,
    pub addons: Option<HashMap<String, Addon>>,
}

impl GlobalConfig {
    /// # FIXME
    ///
    /// <https://github.com/rust-lang/rust/issues/35428>
    ///
    /// `remove_empty_and_dedup()` can be generalised.
    pub fn sanitize(mut self) -> Option<GlobalConfig> {
        sanitize_option_vec_string(&mut self.excluded_domains);
        sanitize_option_vec_string(&mut self.excluded_extensions);

        if let Some(ect) = self.excluded_content_types.as_mut() {
            ect.sort();
            ect.dedup();
            ect.retain(|ct| ct != &ContentType::Unknown);
        };

        if self.excluded_content_types.is_some()
            || self.excluded_domains.is_some()
            || self.excluded_extensions.is_some()
            || self.with_ws.is_some()
            || self.addons.is_some()
        {
            return Some(self);
        }
        None
    }

    pub fn parse_addons(&mut self) -> Option<HashMap<String, Addon>> {
        self.addons.take()
    }
}

static GLOBAL_CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();

// $HOME/.config/zxc
pub fn global_config_path() -> Result<&'static Path, VarError> {
    if let Some(path) = GLOBAL_CONFIG_DIR.get() {
        return Ok(path.as_path());
    }

    let home_dir = env::var("HOME")?;
    let mut path = PathBuf::from(home_dir);
    path.push(".config/zxc");
    Ok(GLOBAL_CONFIG_DIR.get_or_init(|| path).as_path())
}

/// Parses the global config and sanitizes it.
pub fn parse_global_config() -> Result<Option<GlobalConfig>, ConfigError> {
    let path = global_config_path()?.join(CONFIG_FILE_NAME);
    let contents = read_to_string(&path)?;
    let config = toml::from_str::<GlobalConfig>(&contents)?;
    Ok(config.sanitize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_config() {
        let config_toml = r#"
            with_ws = true
            excluded_domains = ["*mozilla*", "*firefox*"]
            excluded_content_types = ["audio", "font", "img", "video"]
            excluded_extensions = ["css", "js"]

            [addons]
            [addons.ffuf]
            prefix = "z-"
            request_flag = "-request"
            http_flag = "-request-proto http"
            http2_flag = "-http2"
            add_flag = "-w"

            [addons.sqlmap]
            prefix = "q-"
            request_flag = "-r"
            https_flag = "--force-ssl"
            http2_flag = "--http2"
            add_flag = "--batch --dbs"
            "#;

        let ffuf = addons::tests::build_ffuf();
        let sqlmap = addons::tests::build_sqlmap();

        let gc = toml::from_str::<GlobalConfig>(config_toml).unwrap();
        let verify = GlobalConfig {
            excluded_domains: Some(vec![
                "*mozilla*".to_string(),
                "*firefox*".to_string(),
            ]),
            excluded_content_types: Some(vec![
                ContentType::Audio,
                ContentType::Font,
                ContentType::Image,
                ContentType::Video,
            ]),
            excluded_extensions: Some(vec![
                "css".to_string(),
                "js".to_string(),
            ]),
            with_ws: Some(true),
            addons: Some(HashMap::from([
                ("ffuf".to_string(), ffuf),
                ("sqlmap".to_string(), sqlmap),
            ])),
        };

        assert_eq!(gc, verify);
    }
}
