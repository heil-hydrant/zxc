use serde::{Deserialize, Serialize};

use crate::commander::Codec;
use crate::file_types::FileType;
use crate::forward_info::ForwardInfo;
pub use crate::forward_info::H2Info;
use crate::proxy::{Role, ServerInfoJson};

// ============================================================================
// Messages To UI
// ============================================================================

// Format:
//
//      {   'id': 1,
//          'ft': 'req',
//          'server_info': {'host': 'www.google.com'}
//          'is_h2': true,
//          'h2_info': {
//              'authority': 'www.google.com',
//              'scheme': 'https'
//              }
//          'ws_info': {
//              'log_id': 1
//              'is_bin: true
//              }
//      }

#[derive(Debug, Serialize)]
pub struct InterToUI {
    id: usize,
    ft: FileType,
    #[serde(skip_serializing_if = "Option::is_none")]
    server_info: Option<ServerInfoJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_h2: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    h2_info: Option<H2Info>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ws_info: Option<WsInfo>,
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

#[derive(Debug, Serialize)]
struct WsInfo {
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

// ============================================================================
// Messages From UI
// ============================================================================

#[derive(Deserialize, Debug)]
pub struct InterUImsg {
    id: usize,
    operation: InterUIOps,
}

impl InterUImsg {
    pub fn id(&self) -> usize {
        self.id
    }

    pub fn op(&self) -> &InterUIOps {
        &self.operation
    }
}

impl From<InterUImsg> for InterUIOps {
    fn from(msg: InterUImsg) -> Self {
        msg.operation
    }
}

// Different types of operation from UI
#[derive(Debug, Deserialize)]
pub enum InterUIOps {
    Close,
    Drop(usize, FileType),
    Resume(ResumeInfo),
    Encode {
        codec: Codec,
        data: String,
    },

    Decode {
        codec: Codec,
        data: String,
    },
    Forward(ForwardInfo),
    Toggle,
}

// FileType Specific Resume Information
//
//      request:
//          { req: {
//              need_response: Option<bool>,
//              update: Option<bool>,
//              server_info: {
//                    host: String,
//                    scheme: String,
//                    sni: String },
//              h2_info: {
//                    authority: Option<String>,
//                    scheme: Option<String> } } }
//
//      response:
//          { res: { update: bool } }
//
//      websocket request:
//          { wreq: { need_response: Option<bool> } }
//
//      websocket response:
//           wres

#[derive(Debug, Deserialize)]
enum FtSpecific {
    #[serde(rename = "req")]
    Req {
        server_info: Option<ServerInfoJson>,
        need_response: Option<bool>,
        update: Option<bool>,
        #[serde(default)]
        h2_info: Option<H2Info>,
    },
    #[serde(rename = "res")]
    Res {
        update: Option<bool>,
    },
    #[serde(rename = "wreq")]
    WReq {
        need_response: Option<bool>,
    },
    #[serde(rename = "wres")]
    WRes,
}

// Format:
//
//      common:
//          { id: 0, modified: false, ft: *}
//
//      request:
//          { req: {
//              need_response: Option<bool>,
//              update: Option<bool>,
//              server_info: {
//                    host: String,
//                    scheme: String,
//                    sni: String },
//              h2_info: {
//                    authority: Option<String>,
//                    scheme: Option<String> } } }
//
//      response:
//          { res: { update: bool } }
//
//      websocket request:
//          { wreq: {
//              need_response: Option<bool> } }
//
//      websocket response:
//          { wres }

#[derive(Debug, Deserialize)]
pub struct ResumeInfo {
    pub id: usize,
    modified: Option<bool>,
    ft: FtSpecific,
}

impl ResumeInfo {
    pub fn modified(&self) -> bool {
        self.modified.is_some()
    }

    // For filetypes req and wreq only, if b:need_response is set in ui, true
    // by default false
    pub fn need_response(&self) -> bool {
        match self.ft {
            FtSpecific::Req {
                need_response,
                ..
            }
            | FtSpecific::WReq {
                need_response,
            } => need_response.is_some(),
            _ => false,
        }
    }

    // For filetypes req and res only, if b:update is set in ui, false
    // by default true for ws
    pub fn update(&self) -> bool {
        match self.ft {
            FtSpecific::Req {
                update,
                ..
            }
            | FtSpecific::Res {
                update,
            } => update.is_none(),
            _ => true,
        }
    }

    /// Builds a resume request for a server reconnect.
    ///
    /// When `HandleOneOneError::ReadFromServer` occurs, the proxy
    /// reconnects and resumes from `ProxyState::ReadModFile`, which
    /// requires a `ResumeInfo`. The frame is already updated and rewritten
    /// according to the user, so `update` is set to `Some`; then
    /// `resume_info.update()` returns `false` and the state transitions to
    /// `Send` directly.
    pub fn request() -> Self {
        Self {
            id: 0,
            modified: None,
            ft: FtSpecific::Req {
                server_info: None,
                need_response: None,
                update: Some(true),
                h2_info: None,
            },
        }
    }

    pub fn file_type(&self) -> FileType {
        match self.ft {
            FtSpecific::Req {
                ..
            } => FileType::Req,
            FtSpecific::Res {
                ..
            } => FileType::Res,
            FtSpecific::WReq {
                ..
            } => FileType::Wreq,
            FtSpecific::WRes => FileType::Wres,
        }
    }

    pub fn is_wreq(&self) -> bool {
        matches!(self.ft, FtSpecific::WReq { .. })
    }

    // for ft req only, pseudo-header metadata sent back for HTTP/2 streams
    pub fn h2_info(&self) -> Option<&H2Info> {
        if let FtSpecific::Req {
            h2_info,
            ..
        } = &self.ft
        {
            h2_info
                .as_ref()
                .filter(|info| !info.is_empty())
        } else {
            None
        }
    }

    // for ft req only
    pub fn into_server_info(mut self) -> Option<ServerInfoJson> {
        if let FtSpecific::Req {
            server_info,
            ..
        } = &mut self.ft
        {
            server_info.take()
        } else {
            None
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

    fn deserialize(value: serde_json::Value) -> ResumeInfo {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn test_resume_info_h2_info() {
        let resume_info = deserialize(serde_json::json!({
            "id": 1,
            "modified": true,
            "ft": {
                "req": {
                    "h2_info": {
                        "authority": "internal.local:8443",
                        "scheme": "https"
                    }
                }
            }
        }));
        let h2_info = resume_info.h2_info().unwrap();
        assert_eq!(h2_info.authority(), Some("internal.local:8443"));
        assert_eq!(h2_info.scheme(), Some("https"));
    }

    #[test]
    fn test_resume_info_without_h2_info() {
        let resume_info = deserialize(serde_json::json!({
            "id": 1,
            "ft": {"req": {"update": true}}
        }));
        assert!(resume_info.h2_info().is_none());
    }

    #[test]
    fn test_resume_info_empty_h2_info_is_ignored() {
        let resume_info = deserialize(serde_json::json!({
            "id": 1,
            "ft": {"req": {"h2_info": {}}}
        }));
        assert!(resume_info.h2_info().is_none());
    }

    #[test]
    fn test_resume_info_h2_info_does_not_set_server_info() {
        let resume_info = deserialize(serde_json::json!({
            "id": 1,
            "ft": {
                "req": {
                    "h2_info": {
                        "authority": "internal.local:8443",
                        "scheme": "https"
                    }
                }
            }
        }));
        assert!(resume_info.into_server_info().is_none());
    }
}
