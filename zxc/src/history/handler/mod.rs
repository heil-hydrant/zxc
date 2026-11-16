mod impl_close_action;
mod impl_flush_storage;
mod impl_from_commander;
mod impl_handle_commander_msg;
mod impl_handle_ui;

use std::fmt::Debug;

use bytes::BytesMut;
use rustc_hash::FxHashMap;
use tokio::fs::{File, OpenOptions};
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{error, trace};
use zxc_derive::{Buffer, NotifyCommander};

use crate::CAPACITY_2MB;
use crate::history::{
    CommanderToHistory, HISTORY_WS_HIS, HISTORY_WS_WSESS, HistoryError,
    HistoryUIOps, HistoryWsError, HistoryWsRegisterInfo,
};
use crate::id::SoldierId;
use crate::io::write_and_flush;
use crate::run::boundary::{
    Buffer, HandleCommander, NotifyCommander, UiReply,
};

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
    ws_storage: FxHashMap<SoldierId, File>, // Stores the Ws History
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
            ws_storage: FxHashMap::default(),
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
     *          a. If get_response errors, log it and continue so the batch
     *             is fully drained without leaving reversed leftovers.
     *
     *          b. If with_sock, return UiReply::Flush, to write_vectored()
     */
    pub async fn handle_commander(
        &mut self,
        size: usize,
        with_sock: bool,
    ) -> Result<UiReply, HistoryError> {
        if size == 1 {
            trace!("recv| 1");
            let msg = self.msg_storage.pop().unwrap();
            if let Some(res) = self.get_response(msg).await? {
                if with_sock {
                    return Ok(UiReply::push(res));
                } else {
                    self.ui_storage.push(res);
                }
            }
        } else {
            trace!("recv| {}", size);
            self.msg_storage.reverse();
            while let Some(msg) = self.msg_storage.pop() {
                match self.get_response(msg).await {
                    Ok(Some(res)) => self.ui_storage.push(res),
                    Ok(None) => (),
                    Err(e) => error!("history| {}", e),
                }
            }
            if with_sock {
                return Ok(UiReply::Flush);
            }
        };
        Ok(UiReply::None)
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
             *      3. Open the .wsess file which maps id to the WebSocket
             *         history and add it to self.ws_storage
             *
             * Error:
             *      HistoryWsError::FileCreate [1]
             *      HistoryWsError::Write      [2]
             *      HistoryWsError::Register   [3]
             */
            CommanderToHistory::RegisterWs(reginfo) => {
                trace!("ws register");
                let rply = if self.ws_his.is_none() {
                    let hfile = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(HISTORY_WS_HIS)
                        .await
                        .map_err(HistoryWsError::FileCreate)?;
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
                .map_err(HistoryWsError::Write)?;

                let id = reginfo.id;
                let wsfile = open_ws_history(reginfo)
                    .await
                    .map_err(HistoryWsError::Register)?;
                self.ws_storage.insert(id, wsfile);
                rply
            }

            /* Associcated Values:
             *      id      : usize
             *      logdata : String
             *
             * Error:
             *      HistoryError::NoId      [1]
             *      HistoryWsError::Write   [2]
             */
            CommanderToHistory::WebSocket(id, logdata) => {
                trace!("ws write");
                let wslog = self
                    .ws_storage
                    .get_mut(&id)
                    .ok_or(HistoryError::NoId(id, WS_HISTORY))?;
                write_and_flush(wslog, logdata.as_bytes())
                    .await
                    .map_err(HistoryWsError::Write)?;
                None
            }

            /* Associcated Values:
             *      id : usize
             *
             * Error:
             *      HistoryError::NoId  [1]
             */
            CommanderToHistory::RemoveWs(id) => {
                self.ws_storage
                    .remove(&id)
                    .ok_or(HistoryError::NoId(id, WS_REMOVE))?;
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

pub async fn open_ws_history(
    mut info: HistoryWsRegisterInfo,
) -> Result<File, std::io::Error> {
    info.path.push(HISTORY_WS_WSESS);
    OpenOptions::new()
        .append(true)
        .create(true)
        .open(&info.path)
        .await
}
