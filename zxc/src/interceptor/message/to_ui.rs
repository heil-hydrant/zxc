use serde::{Deserialize, Serialize};

use crate::file_types::FileType;
use crate::proxy::{Role, ServerInfoJson};

// Different types of msgs that can be sent to Interceptor UI
//
// Format:
//
//      {   'id': 1,
//          'ft': 'req',
//          'server_info': {'host': 'www.google.com'}
//          'ws_info': {
//              'log_id': 1
//              'is_bin: true
//              }
//          'h2_info': {
//              'authority': 'www.google.com',
//              'scheme': 'https'
//              }
//          }

#[derive(Debug, Serialize)]
pub struct InterToUI {
    id: usize,
    ft: FileType,
    #[serde(skip_serializing_if = "Option::is_none")]
    server_info: Option<ServerInfoJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_h2: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ws_info: Option<WsInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    h2_info: Option<H2Info>,
}

impl InterToUI {
    pub fn build_http_req(
        id: usize,
        server_info: Option<ServerInfoJson>,
    ) -> Self {
        Self {
            id,
            ft: FileType::Req,
            server_info,
            is_h2: None,
            ws_info: None,
            h2_info: None,
        }
    }

    // HTTP/2 requests carry pseudo-header metadata that cannot be represented
    // in the origin-form request logged to disk.
    pub fn build_h2_req(
        id: usize,
        server_info: Option<ServerInfoJson>,
        h2_info: Option<H2Info>,
    ) -> Self {
        Self {
            id,
            ft: FileType::Req,
            server_info,
            is_h2: Some(true),
            ws_info: None,
            h2_info,
        }
    }

    pub fn build_http_res(id: usize) -> Self {
        Self {
            id,
            ft: FileType::Res,
            server_info: None,
            is_h2: None,
            ws_info: None,
            h2_info: None,
        }
    }

    pub fn build_ws(
        id: usize,
        log_id: usize,
        role: &Role,
        is_bin: bool,
    ) -> Self {
        let ft = role.as_ws_ft();
        let ws_info = Some(WsInfo::new(log_id, is_bin));
        Self {
            id,
            ft,
            server_info: None,
            is_h2: None,
            ws_info,
            h2_info: None,
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn is_http(&self) -> bool {
        matches!(self.ft, FileType::Req) || matches!(self.ft, FileType::Res)
    }

    pub fn is_wreq(&self) -> bool {
        matches!(self.ft, FileType::Wreq)
    }

    pub fn file_type(&self) -> &FileType {
        &self.ft
    }
}

// HTTP/2 pseudo-header metadata. Both fields are optional: if the request
// does not carry them, nothing is sent to the UI.
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
}

#[derive(Debug, Serialize)]
pub struct WsInfo {
    log_id: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_bin: Option<bool>,
}

impl WsInfo {
    fn new(log_id: usize, is_bin: bool) -> Self {
        WsInfo {
            log_id,
            is_bin: if is_bin {
                Some(true)
            } else {
                None
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::server_info::scheme::Scheme;

    fn h2_info() -> H2Info {
        H2Info::new(
            Some("www.google.com".to_string()),
            Some("https".to_string()),
        )
    }

    #[test]
    fn test_build_h2_req_serializes_pseudo_headers() {
        let server_info = ServerInfoJson::new(
            "www.google.com".to_string(),
            Scheme::Https,
            None,
        );
        let msg =
            InterToUI::build_h2_req(1, Some(server_info), Some(h2_info()));
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["ft"], "req");
        assert_eq!(json["is_h2"], true);
        assert_eq!(json["h2_info"]["authority"], "www.google.com");
        assert_eq!(json["h2_info"]["scheme"], "https");
    }

    #[test]
    fn test_build_h2_req_omits_missing_pseudo_headers() {
        let msg = InterToUI::build_h2_req(1, None, None);
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["is_h2"], true);
        assert!(json.get("h2_info").is_none());
        assert!(json.get("server_info").is_none());
    }

    #[test]
    fn test_h2_info_skips_empty_fields() {
        let info = H2Info::new(Some("www.google.com".to_string()), None);
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json, serde_json::json!({"authority": "www.google.com"}));
    }

    #[test]
    fn test_build_http_req_has_no_h2_info() {
        let msg = InterToUI::build_http_req(2, None);
        let json = serde_json::to_value(&msg).unwrap();
        assert!(json.get("h2_info").is_none());
        assert!(json.get("is_h2").is_none());
    }
}
