use std::path::PathBuf;

use bytes::BytesMut;
use rustls_pki_types::ServerName;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::commander::CommanderResponse;
use crate::history::message::from_commander::CommanderToHistory;
use crate::proxy::server_info::ServerInfo;
use crate::proxy::server_info::address::Address;

/* When a new connection is required, OneOneStruct<Request> is split into
 * Connection and AdditionalHandlerInfo structs.
 *
 * AdditionalHandlerInfo contains the required info to rebuild the
 * OneOneStruct<Request>
 */

pub struct AdditionalHandlerInfo {
    pub log_id: usize,
    pub payload: BytesMut,
    pub path: PathBuf,
    pub should_intercept: bool,
    pub receiver: Receiver<CommanderResponse>,
    pub server_info: ServerInfo,
    pub history_sender: Option<Sender<CommanderToHistory>>,
}

impl AdditionalHandlerInfo {
    pub fn new(
        log_id: usize,
        payload: BytesMut,
        path: PathBuf,
        need_response: bool,
        receiver: Receiver<CommanderResponse>,
        server_info: ServerInfo,
        history_sender: Option<Sender<CommanderToHistory>>,
    ) -> Self {
        Self {
            log_id,
            payload,
            path,
            should_intercept: need_response,
            receiver,
            server_info,
            history_sender,
        }
    }

    pub fn address(&self) -> &Address {
        self.server_info.address()
    }

    pub fn sni(&self) -> &ServerName<'_> {
        self.server_info.sni()
    }
}
