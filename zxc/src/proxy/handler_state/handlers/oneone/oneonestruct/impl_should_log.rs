use std::path::{Path, PathBuf};
use std::str::FromStr;

use header_plz::Method;
use http::uri::PathAndQuery;
use mime_plz::ContentType;
use tokio::sync::mpsc::Sender;
use tracing::{error, trace};

use super::*;
use crate::CommanderRequest;
use crate::commander::CommanderResponse;
use crate::commander::communicate::response::convert::WrongMessage;
use crate::history::message::from_commander::CommanderToHistory;
use crate::proxy::handler_state::ShouldLog;

const ACCEPT: &str = "Accept";

impl<T, E> ShouldLog for OneOneStruct<T, E, OneRequestLine> {
    type LogResult = (usize, PathBuf, Sender<CommanderToHistory>);

    /* Steps:
     *      1. Get the method as enum.
     *
     *      2. Match method.
     *          a. If GET,
     *              1. Get the uri as string by calling uri_as_string().
     *
     *              2. Get the extension from the uri by building a
     *                 PathAndQuery from the uri.
     *
     *              3. if path is Ok(), return extension else empty string.
     *
     *              4. If no extension or emty string,
     *                  a. Get the accept header value by calling
     *                  value_for_key(ACCEPT)
     *
     *                  b. If Some(), convert it to ContentType calling
     *                  ContentType::from_accept_header().
     *
     *                  c. If None, return Some("").
     *
     *                  d. If ContentType in Some, return
     *                  CommanderRequest::ShouldLogHttpCt(id, content_type).
     *
     *          b. If HEAD, OPTIONS, TRACE , return None.
     *
     *          c. Else return Some("").
     *
     *      3. Default, CommanderRequest::ShouldLogHttp(id, ext).
     *
     * Returns:
     *      Option<CommanderRequest::ShouldLogHttp (id, ext)> |
     *          Option<CommanderRequest::ShouldLogHttpCt (id, ext)>
     */

    fn get_log_request(&self) -> Option<CommanderRequest> {
        let frame = self.frame.as_ref().unwrap(); // safe to unwrap
        let method = frame.method_enum();

        let ext = match method {
            // 2.a. If GET
            Method::GET => {
                let uri = frame.uri_as_string();
                trace!("URI| {}", uri);

                let ext = match PathAndQuery::from_str(&uri) {
                    Ok(uri) if uri.path() != "/" => Path::new(uri.path())
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e.to_string())
                        .unwrap_or_default(),
                    Err(e) => {
                        error!("uri parse: {}", e);
                        "".to_string()
                    }
                    Ok(_) => "".to_string(),
                };
                trace!("extension| {}", ext);
                // if ext is empty string, check if there is accept header
                if ext.is_empty() {
                    trace!("no ext");
                    if let Some(ct) = frame
                        .header_map()
                        .value_of_key(ACCEPT)
                        .and_then(|bytes| std::str::from_utf8(bytes).ok())
                        .and_then(ContentType::from_accept_header)
                    {
                        trace!("accept header| {}", ct);
                        return Some(CommanderRequest::ShouldLogHttpCt(
                            self.id, ct,
                        ));
                    }
                    trace!("no accept header");
                }
                Some(ext)
            }
            Method::HEAD | Method::OPTIONS | Method::TRACE => return None,
            _ => Some("".to_string()),
        };

        ext.map(|ext| CommanderRequest::ShouldLogHttp(self.id, ext))
    }

    /* Steps:
     *      Use TryFrom trait implementation in communicate/response/convert.rs
     *      to convert CommanderResponse::HttpLog to
     *      Option<(usize, PathBuf, Sender<CommanderToHistory>)>
     */

    fn parse_log_response(
        &self,
        response: CommanderResponse,
    ) -> Result<Option<Self::LogResult>, WrongMessage> {
        Option::<(usize, PathBuf, Sender<CommanderToHistory>)>::try_from(
            response,
        )
    }

    fn update_path(
        &mut self,
        (index, path, history_sender): (
            usize,
            PathBuf,
            Sender<CommanderToHistory>,
        ),
    ) {
        self.log_id = index;
        self.path = Some(path);
        self.history_sendr = Some(history_sender);
    }
}

const SHOULD_LOG_PANIC: &str = "shouldlog| not applicable for response";

// Blanket implementation Not Applicable for Response.
// should succeed in can_log()
impl<T, E> ShouldLog for OneOneStruct<T, E, OneResponseLine> {
    type LogResult = ();

    fn parse_log_response(
        &self,
        _: CommanderResponse,
    ) -> Result<Option<Self::LogResult>, WrongMessage> {
        panic!("{}", SHOULD_LOG_PANIC)
    }

    fn update_path(&mut self, _: Self::LogResult) {
        panic!("{}", SHOULD_LOG_PANIC)
    }

    fn get_log_request(&self) -> Option<CommanderRequest> {
        panic!("{}", SHOULD_LOG_PANIC)
    }
}
