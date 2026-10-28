use std::path::PathBuf;

use bytes::BytesMut;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{Sink, SinkExt, Stream, StreamExt};
use tokio::fs::File;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::{self, Message};
use tracing::trace;

use super::WsConn;
use crate::commander::*;
use crate::history::*;
use crate::interceptor::*;
use crate::proxy::handler_state::transition::write_history::WsHistory;
use crate::proxy::prelude::*;

// --- AddRaw ---
// Blank implementation. No add_raw for ws
impl<T, E> AddRaw for WsConn<T, E> {
    #[inline(always)]
    fn add_raw(&mut self, _: BytesMut) {
        panic!("{}", WS_ADD_RAW_PANIC);
    }
}

// --- BytesToMsg ---
/* Steps:
 *      Only text and binary frame is supported. build new frame of the same
 *      type. For other frame, return an error.
 *
 * Errors:
 *      ProxyUpdateFrameError::InvalidWsFrame
 */

impl<T, E> BytesToMsg for WsConn<T, E> {
    type Msg = Message;

    fn parse_bytes(&self, buf: BytesMut) -> Result<Self::Msg, UpdateMsgError> {
        // safe to unwrap
        let new_frame = match self.msg.as_ref().unwrap() {
            Message::Text(_) => Message::Text(
                String::from_utf8_lossy(&buf)
                    .to_string()
                    .into(),
            ),
            Message::Binary(_) => Message::Binary(buf.into()),
            _ => return Err(UpdateMsgError::InvalidWsFrame),
        };
        Ok(new_frame)
    }

    #[inline(always)]
    fn add_msg(&mut self, msg: Self::Msg) {
        self.msg = Some(msg);
    }
}

// --- DropMsg ---
// For websocket, continue to receive message
impl<T, E> DropMsg for WsConn<T, E> {
    #[inline(always)]
    fn continue_on_drop() -> Result<(), ProxyStateError> {
        Ok(())
    }
}

// --- MsgToBytes ---
// Blanket implementation
impl<T, E> MsgToBytes for WsConn<T, E> {
    #[inline(always)]
    fn msg_to_bytes(&mut self) {}
}

// --- Intercept ---
impl<T, E> Intercept for WsConn<T, E> {
    fn get_inter_info(&self) -> InterToUI {
        InterToUI::build_ws(
            self.http_id,
            self.log_id,
            &self.role,
            self.msg.as_ref().unwrap().is_binary(),
        )
    }
}

// --- Log & UpdateLogExt ---
impl<T, E> Log for WsConn<T, E> {
    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn log_data(&self) -> &[u8] {
        match self
            .msg
            .as_ref()
            .unwrap() // safe to unwrap
        {
            Message::Text(data) => data.as_bytes(),
            Message::Binary(vec) => vec,
            _ => unreachable!(),
        }
    }
}

impl<T, E> UpdateLogExt for WsConn<T, E> {
    fn update_extension(&mut self) {
        self.path
            .set_file_name(self.log_id.to_string());
        self.path
            .set_extension(self.role.ws_ext());
    }
}

// --- ReadWrite ---
impl<T, E> ReadWrite for WsConn<T, E>
where
    SplitSink<WebSocketStream<E>, Message>: Sink<Message>,
    SplitStream<WebSocketStream<T>>:
        Stream<Item = Result<Message, tungstenite::Error>>,
{
    type Error = WsError;
    type State = ProxyState<Self>;

    async fn read(mut self) -> Result<ProxyState<Self>, WsError> {
        if let Some(msg) = self.reader.next().await {
            self.msg = Some(msg?);
        }
        trace!("Y");
        Ok(ProxyState::ShouldLog(self))
    }

    async fn write(mut self) -> Result<ProxyState<Self>, WsError> {
        let data = self.msg.take().unwrap(); // safe to unwrap
        let is_close = data.is_close();
        self.writer
            .send(data)
            .await
            .map_err(|_| WsError::Write)?;
        trace!("Y");
        if !is_close {
            trace!("continue");
            return Ok(ProxyState::Receive(self));
        }
        trace!("close frame");
        Ok(ProxyState::End(self))
    }
}

// --- Rewrite ---
impl<T, E> Rewrite for WsConn<T, E> {
    #[inline(always)]
    fn file_and_data(&mut self) -> (&mut File, &[u8]) {
        panic!("No rewrite for WsHandle");
    }
}

// --- ShouldIntercept ---
#[inline(always)]
const fn ws_should_intercept(role: &Role) -> Option<bool> {
    if let Role::Server = role {
        Some(true)
    } else {
        None
    }
}

impl<T, E> ShouldIntercept for WsConn<T, E> {
    #[inline(always)]
    fn should_intercept(&self) -> Option<bool> {
        ws_should_intercept(&self.role)
    }

    async fn query_commander_should_intercept(
        &mut self,
    ) -> Result<bool, ProxyStateError> {
        if let Role::Client = self.role {
            let req = CommanderRequest::ShouldInterceptWsRespone(self.id);
            self.commander_sendr.send(req).await?;
            let result = self
                .commander_recvr
                .recv()
                .await
                .ok_or(ProxyStateError::WsShouldIntercept)?;
            Ok(matches!(result, CommanderResponse::WsInterceptReply(true)))
        } else {
            Ok(false)
        }
    }
}

// --- ShouldLog ---
// Always returns false.
// Each request/response needs to have unique path.
// $id.wreq/wres
impl<T, E> CanLog for WsConn<T, E> {
    #[inline(always)]
    fn can_log(&self) -> bool {
        false
    }
}

impl<T, E> ShouldLog for WsConn<T, E> {
    type LogResult = usize;

    fn commander_log_request(&self) -> Option<CommanderRequest> {
        if self.msg.as_ref().unwrap().is_text()
            || self.msg.as_ref().unwrap().is_binary()
        {
            return Some(CommanderRequest::WsLog(self.id, self.role));
        }
        None
    }

    #[inline(always)]
    fn parse_log_response(
        &self,
        response: CommanderResponse,
    ) -> Result<Option<usize>, WrongMessage> {
        Option::<usize>::try_from(response)
    }

    #[inline(always)]
    fn update_path(&mut self, id: Self::LogResult) {
        self.log_id = id;
    }
}

// --- ShouldRewrite ---
/* Steps:
 *      Always return false as no additional data is added.
 */

impl<T, E> ShouldRewrite for WsConn<T, E> {
    #[inline(always)]
    fn should_rewrite(&self, _: &ResumeInfo) -> bool {
        false
    }
}

// --- UpdateResumeInfo ---
// Blanket implementation
impl<T, E> UpdateResumeInfo for WsConn<T, E> {
    #[inline(always)]
    fn update_resume_info(&mut self, _: &ResumeInfo) {}
}

// --- History ---
impl<T, E> SendHistory for WsConn<T, E> {
    fn get_sender(&self) -> &Sender<CommanderToHistory> {
        &self.history_sendr
    }
}

impl<T, E> GetHistory for WsConn<T, E> {
    fn get_history(&self) -> HistoryEnum<'_> {
        let ws_his = WsHistory::new(
            self.log_id,
            &self.role,
            self.msg.as_ref().unwrap().is_binary(),
            self.msg.as_ref().unwrap().len(), // safe to unwrap
        );

        HistoryEnum::WebSocket(self.id, ws_his)
    }
}
