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

/* Steps:
 *      1. Get Host and Scheme
 *      2. If scheme is Https and should_add_sni() (i.e. host != sni) is true,
 *         get Sni
 *      3. Build ServerInfoJson.
 */

impl From<&ServerInfo> for ServerInfoJson {
    fn from(server_info: &ServerInfo) -> Self {
        let host = server_info.address_to_string();
        let scheme = server_info.scheme();
        let sni = if scheme == Scheme::Https && server_info.should_add_sni() {
            Some(server_info.sni().to_str().to_string())
        } else {
            None
        };

        ServerInfoJson::new(host, scheme, sni)
    }
}
