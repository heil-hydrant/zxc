use serde::Serialize;

use crate::file_types::FileType;
use crate::proxy::handler_state::role::Role;
use crate::proxy::server_info::json::ServerInfoJson;

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
//          }

#[derive(Debug, Serialize)]
pub struct InterToUI {
    id: usize,
    ft: FileType,
    #[serde(skip_serializing_if = "Option::is_none")]
    server_info: Option<ServerInfoJson>,
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
            ws_info: None,
        }
    }

    pub fn build_http_res(id: usize) -> Self {
        Self {
            id,
            ft: FileType::Res,
            server_info: None,
            ws_info: None,
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
            ws_info,
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
