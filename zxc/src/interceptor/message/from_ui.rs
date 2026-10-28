use serde::Deserialize;

use super::to_ui::H2Info;
use crate::commander::Codec;
use crate::file_types::FileType;
use crate::forward_info::ForwardInfo;
use crate::proxy::ServerInfoJson;

// Message from UI
#[derive(Deserialize, Debug)]
pub struct InterUImsg {
    pub id: usize,
    pub operation: InterUIOps,
}

impl InterUImsg {
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
pub enum FtSpecific {
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

    // When HandleOneOneError::ReadFromServer occurs, reconnect to server and
    // resume from ProxyState::ReadModFile state which requires a ResumeInfo
    //
    // As the frame is already updated and rewritten according to the user.
    //
    // By setting update to some, resume_info.update() returns false, so the
    // state can transition to Send directly
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
