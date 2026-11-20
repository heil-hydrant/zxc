use serde::{Deserialize, Serialize};

use crate::proxy::prelude::*;

// Sent and received from ui
#[cfg_attr(test, derive(Default))]
// #[derive(Debug, Serialize, Deserialize)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerInfoJson {
    pub host: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sni: Option<String>,
}

impl ServerInfoJson {
    pub fn new(host: String, scheme: Scheme, sni: Option<String>) -> Self {
        Self {
            host,
            http: scheme.http_as_opt(),
            sni,
        }
    }

    pub fn is_tls(&self) -> bool {
        self.http.is_none()
    }
}

// Builds the UI and history JSON view of `ServerInfo`.
//
// # Steps
//
// 1. Get the host and scheme.
// 2. For HTTPS where the host differs from the SNI, include the SNI.
// 3. Build `ServerInfoJson`.

impl From<&ServerInfo> for ServerInfoJson {
    fn from(server_info: &ServerInfo) -> Self {
        let host = server_info.address_to_string();
        let scheme = server_info.scheme();
        let sni = if scheme == Scheme::Https && server_info.should_add_sni() {
            server_info
                .sni()
                .map(|sni| sni.to_str().to_string())
        } else {
            None
        };

        ServerInfoJson::new(host, scheme, sni)
    }
}
