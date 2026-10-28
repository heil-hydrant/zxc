use std::borrow::Cow;
use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::history::*;
use crate::proxy::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub enum HistoryEnum<'a> {
    Request(RequestHistory<'a>),
    Response(ResponseHistory),
    #[serde(skip)]
    WebSocket(SoldierId, WsHistory),
}

impl<'a> HistoryEnum<'a> {
    pub fn id(&self) -> Option<usize> {
        match self {
            Self::Request(req) => Some(req.id),
            Self::Response(resp) => Some(resp.id),
            Self::WebSocket(..) => None,
        }
    }

    pub fn http_request<M, U>(
        id: usize,
        server_info: &ServerInfo,
        method: M,
        uri: U,
    ) -> Self
    where
        M: Into<Cow<'a, str>>,
        U: Into<Cow<'a, str>>,
    {
        Self::Request(RequestHistory::new(
            id,
            method.into(),
            server_info.scheme(),
            server_info.address_to_string(),
            uri.into(),
        ))
    }

    pub fn http_response(
        id: usize,
        status: u16,
        content_length: Option<&[u8]>,
    ) -> Self {
        let length = content_length
            .and_then(|b| std::str::from_utf8(b).ok())
            .and_then(|s| s.trim().parse::<usize>().ok())
            .unwrap_or(0);
        Self::Response(ResponseHistory::new(id, status, length))
    }
}

impl TryFrom<HistoryEnum<'_>> for CommanderToHistory {
    type Error = serde_json::Error;

    fn try_from(value: HistoryEnum<'_>) -> Result<Self, Self::Error> {
        match value {
            HistoryEnum::Request(_) | HistoryEnum::Response(_) => {
                let res = serde_json::to_string(&value)?;
                Ok(CommanderToHistory::Http(res))
            }
            HistoryEnum::WebSocket(id, ws) => {
                Ok(CommanderToHistory::WebSocket(id, ws.to_string()))
            }
        }
    }
}

// {"Request":{"id":1,"method":"GET","http":bool,"host":"www.google.com"," uri":"/robots.txt"}}
#[derive(Debug, Serialize, Deserialize)]
pub struct RequestHistory<'a> {
    id: usize,
    method: Cow<'a, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    http: Option<bool>,
    host: String,
    uri: Cow<'a, str>,
}

impl<'a> RequestHistory<'a> {
    pub fn new(
        id: usize,
        method: Cow<'a, str>,
        scheme: Scheme,
        host: String,
        uri: Cow<'a, str>,
    ) -> RequestHistory<'a> {
        RequestHistory {
            id,
            http: scheme.http_as_opt(),
            method,
            host,
            uri,
        }
    }
}

// {"Response":{"id":0,"status":"200","length":2000,"mime":"img"}}
#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseHistory {
    id: usize,
    status: u16,
    length: usize,
}

impl ResponseHistory {
    pub fn new(id: usize, status: u16, length: usize) -> ResponseHistory {
        ResponseHistory {
            id,
            status,
            length,
        }
    }
}

#[derive(Debug)]
pub struct WsHistory {
    id: usize,
    is_bin: bool,
    sign: &'static str,
    size: usize,
}

impl WsHistory {
    pub fn new(
        id: usize,
        role: &Role,
        is_bin: bool,
        size: usize,
    ) -> WsHistory {
        WsHistory {
            id,
            is_bin,
            sign: role.as_arrow(),
            size,
        }
    }
}

impl Display for WsHistory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_bin {
            writeln!(f, "{} | {} | b | {}", self.id, self.sign, self.size)
        } else {
            writeln!(f, "{} | {} | {}", self.id, self.sign, self.size)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_history_http() {
        let req_history = RequestHistory::new(
            1,
            std::borrow::Cow::Borrowed("GET"),
            Scheme::Http,
            "www.google.com".to_owned(),
            "/robots.txt".to_owned().into(),
        );
        let his = HistoryEnum::Request(req_history);
        let out = serde_json::to_string(&his).unwrap();
        assert_eq!(
            out,
            r#"{"Request":{"id":1,"method":"GET","http":true,"host":"www.google.com","uri":"/robots.txt"}}"#
        );
    }

    #[test]
    fn test_request_history_https() {
        let req_history = RequestHistory::new(
            1,
            std::borrow::Cow::Borrowed("GET"),
            Scheme::Https,
            "www.google.com".to_owned(),
            "/robots.txt".to_owned().into(),
        );
        let his = HistoryEnum::Request(req_history);
        let out = serde_json::to_string(&his).unwrap();
        assert_eq!(
            out,
            r#"{"Request":{"id":1,"method":"GET","host":"www.google.com","uri":"/robots.txt"}}"#
        );
    }

    #[test]
    fn test_response_history() {
        let res_history = ResponseHistory::new(0, 200, 2000);
        let his = HistoryEnum::Response(res_history);
        let out = serde_json::to_string(&his).unwrap();
        assert_eq!(out, r#"{"Response":{"id":0,"status":200,"length":2000}}"#)
    }

    #[test]
    fn test_ws_history_binary() {
        let ws_history = WsHistory::new(0, &Role::Client, true, 100);
        let out = ws_history.to_string();
        assert_eq!(out, "0 | <- | b | 100\n");
    }

    #[test]
    fn test_ws_history_text() {
        let ws_history = WsHistory::new(0, &Role::Server, false, 100);
        let out = ws_history.to_string();
        assert_eq!(out, "0 | -> | 100\n");
    }
}
