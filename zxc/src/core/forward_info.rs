use std::ffi::OsStr;
use std::path::PathBuf;

use header_plz::uri::InvalidUri;
use http_plz::Request;
use serde::{Deserialize, Serialize};

use crate::proxy::server_info::json::ServerInfoJson;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct H2Info {
    #[serde(skip_serializing_if = "Option::is_none")]
    authority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scheme: Option<String>,
}

impl H2Info {
    pub fn new(authority: Option<String>, scheme: Option<String>) -> Self {
        Self {
            authority,
            scheme,
        }
    }

    pub fn authority(&self) -> Option<&str> {
        self.authority.as_deref()
    }

    pub fn scheme(&self) -> Option<&str> {
        self.scheme.as_deref()
    }

    pub fn is_empty(&self) -> bool {
        self.authority.is_none() && self.scheme.is_none()
    }

    pub fn apply_to_request(
        &self,
        req: &mut Request,
    ) -> Result<(), InvalidUri> {
        if let Some(authority) = self.authority() {
            req.try_set_authority(authority)?;
        }
        if let Some(scheme) = self.scheme() {
            req.try_set_scheme(scheme)?;
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Module {
    Repeater,
    Addon(String),
}

// Message that is sent between UI
#[derive(Serialize, Deserialize, Debug)]
pub struct ForwardInfo {
    #[serde(skip_serializing)]
    to: Module,
    pub file: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_info: Option<ServerInfoJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_h2: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h2_info: Option<H2Info>,
}

impl ForwardInfo {
    pub fn to_module(&self) -> &Module {
        &self.to
    }

    pub fn file_extension(&self) -> Option<&OsStr> {
        self.file.extension()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forward_info_serializes_h2_metadata() {
        let info = ForwardInfo {
            to: Module::Repeater,
            file: PathBuf::from("history/1/1.req"),
            server_info: None,
            is_h2: Some(true),
            h2_info: Some(H2Info::new(
                Some(String::new()),
                Some("https".to_string()),
            )),
        };

        let value = serde_json::to_value(&info).unwrap();

        assert!(value.get("to").is_none());
        assert_eq!(value["is_h2"], true);
        assert_eq!(value["h2_info"]["authority"], "");
        assert_eq!(value["h2_info"]["scheme"], "https");
    }

    #[test]
    fn test_forward_info_deserializes_h2_metadata() {
        let value = serde_json::json!({
            "to": "Repeater",
            "file": "history/1/1.req",
            "is_h2": true,
            "h2_info": {
                "authority": "",
                "scheme": "https"
            }
        });

        let info: ForwardInfo = serde_json::from_value(value).unwrap();

        assert_eq!(info.is_h2, Some(true));
        let h2_info = info.h2_info.unwrap();
        assert_eq!(h2_info.authority(), Some(""));
        assert_eq!(h2_info.scheme(), Some("https"));
    }
}
