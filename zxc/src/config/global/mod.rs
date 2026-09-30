use std::collections::HashMap;

use addons::Addon;
use mime_plz::ContentType;
use serde::Deserialize;
pub mod parser;

use super::misc::sanitize_option_vec_string;

pub mod addons;

// Global config only contains a list of excluded domains and excluded content
// types
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
    /* Steps:
     *      1. Remove empty values from excluded_domains, excluded_extensions
     *      and excluded_content_types.
     *
     *      2. Remove ContentType::Unknown from excluded_content_types.
     *
     *      FIX:
     *          https://github.com/rust-lang/rust/issues/35428
     *          remove_empty_and_dedup() can be generalised
     *
     *      3. If all fields are empty, return None
     */

    pub fn sanitize(mut self) -> Option<GlobalConfig> {
        // 1. Remove empty values
        sanitize_option_vec_string(&mut self.excluded_domains);
        sanitize_option_vec_string(&mut self.excluded_extensions);

        // 2. sort and dedup
        if let Some(ect) = self.excluded_content_types.as_mut() {
            ect.sort();
            ect.dedup();
            // 2. remove ContentType::Unknown
            ect.retain(|ct| ct != &ContentType::Unknown);
        };

        // 3. If all fields are empty, return None
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
            add_flag = "-w"

            [addons.sqlmap]
            prefix = "q-"
            request_flag = "-r"
            https_flag = "--force-ssl"
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
