use tracing::trace;

use super::can_communicate::CanCommunicate;
use super::frame_to_payload::FrameToPayload;
use crate::commander::communicate::response::convert::WrongMessage;
use crate::commander::{CommanderRequest, CommanderResponse};
use crate::proxy::handler_state::{ProxyState, ProxyStateError};

const SHOULD_LOG: &str = "ShouldLog";

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

pub trait ShouldLog {
    type LogResult;

    fn get_log_request(&self) -> Option<CommanderRequest>;

    fn parse_log_response(
        &self,
        response: CommanderResponse,
    ) -> Result<Option<Self::LogResult>, WrongMessage>;

    fn update_path(&mut self, result: Self::LogResult);
}

// Description:
//      Transition function to check if request/response should be logged.
//
// Transition:
//      ShouldLog -> WriteHistory | Send
//
// Steps:
//      1. Check if connection can be logged. If a http request is logged, then
//         the corresponding response can be logged as well. Return
//         ProxyState::WriteHistory
//
//      2. If not, get
//              CommanderRequest::ShouldLogHttp(id, extension) /
//              CommanderRequest::ShouldLogHttpCt(id, content_type) for http
//
//              CommanderRequest::WsLog(id, role) for ws.
//
//         NOTE: HTTP methods HEAD, OPTIONS, TRACE are not logged. So they
//         don't return any query.
//
//          Ws Binary frames are not logged.
//
//      3. If request is some, send the query to the commander and receive the
//         response
//
//      4. Pass the result to parse_log_respone() to get Option<LogResult>
//
//      4. If the result is Some, pass the result to update_path(). return
//         ProxyState::WriteHistory.
//
//      5. If all the above cases fail, convert frame to payload (http only)
//         and return ProxyState::Send. Relay
//
// Returns:
//      Ok(ProxyState::WriteHistory | ProxyState::Send)
//
// Error:
//      ProxyStateError::CommanderRequest       [3]
//      ProxyStateError::CommanderResponse      [3]
//      ProxyStateError::WrongCommand           [4]

pub async fn should_log<T>(
    mut conn: T,
) -> Result<ProxyState<T>, ProxyStateError>
where
    T: CanLog + ShouldLog + FrameToPayload + CanCommunicate,
{
    // 1. HTTP Response only
    if conn.can_log() {
        trace!("can log| Y");
        return Ok(ProxyState::WriteHistory(conn));
    }

    if let Some(req) = conn.get_log_request() {
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

    // 5. Default Case, No logging
    trace!("log| N");
    conn.frame_to_payload();
    Ok(ProxyState::Send(conn))
}
