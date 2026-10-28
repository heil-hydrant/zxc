mod impl_close_action;
mod impl_flush_storage;
mod impl_from_commander;
mod impl_handle_commander_msg;
mod impl_handle_ui;

use std::fmt::Debug;

use bytes::BytesMut;
use tokio::fs::{File, OpenOptions};
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::trace;
use zxc_derive::{Buffer, NotifyCommander};

use crate::CAPACITY_2MB;
use crate::history::{
    CommanderToHistory, HISTORY_WS_HIS, HistoryError, HistoryUIOps, WsHistory,
};
use crate::id::Id;
use crate::io::write_and_flush;
use crate::run::boundary::{Buffer, HandleCommander, NotifyCommander};

const WS_HISTORY: &str = "ws_history";
const WS_REMOVE: &str = "ws_remove";

// History Manager - handles history side of proxy
#[derive(Buffer, NotifyCommander)]
pub struct HistoryHandler {
    buf: BytesMut,
    from_commander: Receiver<CommanderToHistory>,
    msg_storage: Vec<CommanderToHistory>,
    to_commander: Sender<HistoryUIOps>,
    ui_storage: Vec<String>, // Stores the history in case the channel is disconnected
    ws_his: Option<File>,
    ws_storage: Vec<WsHistory>, // Stores the Ws History
}

impl HistoryHandler {
    #[inline(always)]
    pub fn new(
        from_commander: Receiver<CommanderToHistory>,
        to_commander: Sender<HistoryUIOps>,
        storage: Vec<String>,
    ) -> Self {
        Self {
            buf: BytesMut::with_capacity(CAPACITY_2MB),
            from_commander,
            to_commander,
            msg_storage: Vec::with_capacity(100),
            ui_storage: storage,
            ws_his: None,
            ws_storage: Vec::with_capacity(100),
        }
    }

    /* Steps:
     *      1. If size is 1, pop message, get response
     *          a. If with_sock, return Some(response)
     *          b. Else, push to storage
     *
     *      2. Else, reverse msg_storage to maintain order, pop elements and
     *         store response for each message in storage.
     *
     *          a. If with_sock, return Err(HistoryError::NeedsFlush), to
     *             write_vectored()
     */
    pub async fn handle_commander(
        &mut self,
        size: usize,
        with_sock: bool,
    ) -> Result<Option<String>, HistoryError> {
        if size == 1 {
            trace!("recv| 1");
            let msg = self.msg_storage.pop().unwrap();
            if let Some(res) = self.get_response(msg).await? {
                if with_sock {
                    return Ok(Some(res));
                } else {
                    self.ui_storage.push(res);
                }
            }
        } else {
            trace!("recv| {}", size);
            self.msg_storage.reverse();
            while let Some(msg) = self.msg_storage.pop() {
                if let Some(res) = self.get_response(msg).await? {
                    self.ui_storage.push(res);
                }
            }
            if with_sock {
                return Err(HistoryError::NeedsFlush);
            }
        };
        Ok(None)
    }

    pub async fn get_response(
        &mut self,
        msg: CommanderToHistory,
    ) -> Result<Option<String>, HistoryError> {
        let response = match msg {
            /* Associcated Values:
             *      data: String
             */
            CommanderToHistory::Http(data) => Some(data),

            /* Associcated Values:
             *      reginfo: HistoryWsRegisterInfo
             *
             * Steps:
             *      1. First time a ws conn is received,
             *          a. Open ./ws.whis file and store in self.ws_his
             *
             *          b. Send {"Action":"wsview"} to history_ui stream.
             *
             *      2. Get ws history data in format
             *              id | proto | host
             *          by calling log_data() method on WsRegisterInfo and
             *          write to ws.whis
             *
             *      3. Create New WsHistory struct which maps id to .wsess file
             *         and add to self.ws_storage
             *
             * Error:
             *      HistoryError::CreateWs   [1]
             *      HistoryError::WsWrite    [2]
             *      HistoryError::RegisterWs [3]
             */
            CommanderToHistory::RegisterWs(reginfo) => {
                trace!("ws register");
                let rply = if self.ws_his.is_none() {
                    let hfile = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(HISTORY_WS_HIS)
                        .await
                        .map_err(HistoryError::CreateWs)?;
                    self.ws_his = Some(hfile);
                    trace!("ws.whis| attached");
                    // 2
                    let tosend = r#"{"Action":"wsview"}"#;
                    Some(tosend.to_string())
                } else {
                    trace!("ws.whis| exits");
                    None
                };
                write_and_flush(
                    self.ws_his.as_mut().unwrap(), // safe to unwrap
                    reginfo.log_data().as_bytes(),
                )
                .await
                .map_err(HistoryError::WsWrite)?;

                let wshis = WsHistory::new(reginfo)
                    .await
                    .map_err(HistoryError::RegisterWs)?;
                self.ws_storage.push(wshis);
                rply
            }

            /* Associcated Values:
             *      id      : usize
             *      logdata : String
             *
             * Error:
             *      HistoryError::NoId      [1]
             *      HistoryError::WsWrite   [2]
             */
            CommanderToHistory::WebSocket(id, logdata) => {
                trace!("ws write");
                let wslog = self
                    .ws_storage
                    .iter_mut()
                    .find(|wshis| wshis.id() == id)
                    .ok_or(HistoryError::NoId(id, WS_HISTORY))?;
                write_and_flush(wslog.file_as_mut(), logdata.as_bytes())
                    .await
                    .map_err(HistoryError::WsWrite)?;
                None
            }

            /* Associcated Values:
             *      id : usize
             *
             * Error:
             *      HistoryError::NoId  [1]
             */
            CommanderToHistory::RemoveWs(id) => {
                let index = self
                    .ws_storage
                    .iter()
                    .position(|wshis| wshis.id() == id)
                    .ok_or(HistoryError::NoId(id, WS_REMOVE))?;
                self.ws_storage.swap_remove(index);
                trace!("ws removed| {}", id);
                None
            }
        };
        Ok(response)
    }
}

impl Debug for HistoryHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "history")
    }
}
