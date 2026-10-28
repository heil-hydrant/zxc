use std::path::{Path, PathBuf};

use header_plz::Method;
use header_plz::const_headers::ACCEPT;
use header_plz::message_head::header_map::{HMap, Hmap};
use mime_plz::ContentType;
use tokio::sync::mpsc::Sender;
use tracing::trace;

use super::can_communicate::CanCommunicate;
use super::msg_to_bytes::MsgToBytes;
use crate::commander::*;
use crate::history::*;
use crate::id::SoldierId;
use crate::proxy::prelude::*;

const SHOULD_LOG: &str = "ShouldLog";
const SHOULD_LOG_UNREACHABLE: &str = "shouldlog| not applicable for response";

// Trait to check if request/response can be logged.
// http response only
pub trait CanLog {
    fn can_log(&self) -> bool;
}

// Trait to check if request/response should be logged.
//
// Associated type:
//     LogResult   : type of the result of the log query
//
//         http    => (usize, PathBuf, Sender<CommanderToHistory>)
//         ws      => usize

pub fn http_log_request<H, P>(
    id: SoldierId,
    method: &Method,
    headers: &HMap<H>,
    path: impl FnOnce() -> P,
) -> Option<CommanderRequest>
where
    H: Hmap,
    P: AsRef<str>,
{
    let ext = match method {
        Method::GET => {
            let path = path();
            let ext = Path::new(path.as_ref())
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_string)
                .unwrap_or_default();
            trace!("extension| {}", ext);
            if ext.is_empty() {
                trace!("no ext");
                if let Some(ct) = headers
                    .value_of_key(ACCEPT)
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                    .and_then(ContentType::from_accept_header)
                {
                    trace!("accept header| {}", ct);
                    return Some(CommanderRequest::ShouldLogHttpCt(id, ct));
                }
                trace!("no accept header");
            }
            ext
        }
        Method::HEAD | Method::OPTIONS | Method::TRACE => return None,
        _ => String::new(),
    };
    Some(CommanderRequest::ShouldLogHttp(id, ext))
}

pub trait ShouldLog {
    type LogResult;

    fn commander_log_request(&self) -> Option<CommanderRequest> {
        unreachable!("{}", SHOULD_LOG_UNREACHABLE)
    }

    fn parse_log_response(
        &self,
        _response: CommanderResponse,
    ) -> Result<Option<Self::LogResult>, WrongMessage> {
        unreachable!("{}", SHOULD_LOG_UNREACHABLE)
    }

    fn update_path(&mut self, _result: Self::LogResult) {
        unreachable!("{}", SHOULD_LOG_UNREACHABLE)
    }

    #[inline(always)]
    fn needs_msg_to_bytes(&self) -> bool {
        false
    }
}

// Trait for HTTP request handlers that participate in logging.
pub trait HttpRequestLog {
    fn id(&self) -> SoldierId;

    fn log_id_mut(&mut self) -> &mut usize;

    fn path_mut(&mut self) -> &mut Option<PathBuf>;

    fn history_sendr_mut(&mut self)
    -> &mut Option<Sender<CommanderToHistory>>;

    fn http_commander_log_request(&self) -> Option<CommanderRequest>;

    #[inline(always)]
    fn needs_msg_to_bytes(&self) -> bool {
        false
    }
}

impl<T> ShouldLog for T
where
    T: HttpRequestLog,
{
    type LogResult = (usize, PathBuf, Sender<CommanderToHistory>);

    fn commander_log_request(&self) -> Option<CommanderRequest> {
        self.http_commander_log_request()
    }

    fn parse_log_response(
        &self,
        response: CommanderResponse,
    ) -> Result<Option<Self::LogResult>, WrongMessage> {
        response.try_into()
    }

    fn update_path(
        &mut self,
        (index, path, history_sender): (
            usize,
            PathBuf,
            Sender<CommanderToHistory>,
        ),
    ) {
        *self.log_id_mut() = index;
        *self.path_mut() = Some(path);
        *self.history_sendr_mut() = Some(history_sender);
    }

    #[inline(always)]
    fn needs_msg_to_bytes(&self) -> bool {
        <T as HttpRequestLog>::needs_msg_to_bytes(self)
    }
}

pub async fn should_log<T>(
    mut conn: T,
) -> Result<ProxyState<T>, ProxyStateError>
where
    T: CanLog + ShouldLog + MsgToBytes + CanCommunicate,
{
    if conn.can_log() {
        trace!("can log| Y");
        return Ok(ProxyState::WriteHistory(conn));
    }

    if let Some(req) = conn.commander_log_request() {
        trace!("log request| Y");
        conn.sender().send(req).await?;
        let response = conn
            .receiver()
            .recv()
            .await
            .ok_or(ProxyStateError::CommanderResponse(SHOULD_LOG))?;
        if let Some(result) = conn.parse_log_response(response)? {
            trace!("log| Y");
            conn.update_path(result);
            return Ok(ProxyState::WriteHistory(conn));
        }
    }
    trace!("log| N");
    if conn.needs_msg_to_bytes() {
        conn.msg_to_bytes();
    }
    Ok(ProxyState::Send(conn))
}
