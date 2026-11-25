use std::path::PathBuf;

use serde::Deserialize;

use crate::commander::codec::Codec;
use crate::forward_info::{ForwardInfo, H2Info, Module};
use crate::proxy::server_info::json::ServerInfoJson;

/// Represents a message received from the Repeater UI.
///
/// Supported messages:
///
/// [0,{"Encode" : {"Codec": Base64, "data": "aGVsbG8gd29ybGQ="}}]
/// [0,{"Decode" : {"Codec": Base64, "data": "aGVsbG8gd29ybGQ="}}]
/// [0,{"Send" : {  "file": "1.req",
///                 "update": true,
///                 "server_info": {
///                     "scheme": "https",
///                     "host": "www.google.com",
///                     "sni": "www.google.com"     }]]
/// [0,{"WsEstablish": "1.req"}}]
/// [0,{"WsSend": {"id": 1, "is_bin": true}}]
#[derive(Debug, Deserialize)]
pub struct RepeaterMsg {
    pub id: usize,
    pub operation: Operation,
}

impl RepeaterMsg {
    pub fn to_repeater(&self) -> bool {
        if let Operation::Forward(finfo) = &self.operation {
            return matches!(finfo.to_module(), &Module::Repeater);
        }
        false
    }
}

#[derive(Debug, Deserialize)]
pub enum Operation {
    // codec
    Encode {
        codec: Codec,
        data: String,
    },
    Decode {
        codec: Codec,
        data: String,
    },
    // http
    Send(SendInfo),
    // ws
    WsEstablish(SendInfo),
    WsSend {
        id: usize,
        #[serde(default)]
        is_bin: bool,
    },
    WsClose(usize),
    // general
    Forward(ForwardInfo),
    Close,
}

// http request send info
#[derive(Debug, Deserialize)]
pub struct SendInfo {
    pub file: PathBuf,
    pub update: Option<bool>,
    pub server_info: ServerInfoJson,
    pub is_h2: Option<bool>,
    pub h2_info: Option<H2Info>,
}

// WebSocket task internal control message
#[derive(Debug)]
pub enum RepeaterWsMsg {
    Read {
        is_bin: bool,
    },
    Close,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_info_h2_metadata() {
        let value = serde_json::json!([1, {
            "Send": {
                "file": "r-1/rep.req",
                "update": true,
                "server_info": {"host": "example.com:443"},
                "is_h2": true,
                "h2_info": {
                    "authority": "",
                    "scheme": "https"
                }
            }
        }]);
        let msg: RepeaterMsg = serde_json::from_value(value).unwrap();
        let Operation::Send(info) = msg.operation else {
            panic!("expected Send");
        };

        assert_eq!(info.is_h2, Some(true));
        let h2_info = info.h2_info.unwrap();
        assert_eq!(h2_info.authority(), Some(""));
        assert_eq!(h2_info.scheme(), Some("https"));
    }

    #[test]
    fn test_send_info_without_h2_metadata() {
        let value = serde_json::json!([1, {
            "Send": {
                "file": "r-1/rep.req",
                "server_info": {"host": "example.com:443"}
            }
        }]);
        let msg: RepeaterMsg = serde_json::from_value(value).unwrap();
        let Operation::Send(info) = msg.operation else {
            panic!("expected Send");
        };

        assert_eq!(info.is_h2, None);
        assert!(info.h2_info.is_none());
    }

    #[test]
    fn test_ws_send_binary_flag() {
        let value =
            serde_json::json!([1, {"WsSend": {"id": 7, "is_bin": true}}]);
        let msg: RepeaterMsg = serde_json::from_value(value).unwrap();
        let Operation::WsSend {
            id,
            is_bin,
        } = msg.operation
        else {
            panic!("expected WsSend");
        };

        assert_eq!(id, 7);
        assert!(is_bin);
    }

    #[test]
    fn test_ws_send_defaults_to_text() {
        let value = serde_json::json!([1, {"WsSend": {"id": 7}}]);
        let msg: RepeaterMsg = serde_json::from_value(value).unwrap();
        let Operation::WsSend {
            id,
            is_bin,
        } = msg.operation
        else {
            panic!("expected WsSend");
        };

        assert_eq!(id, 7);
        assert!(!is_bin);
    }
}
