mod impl_from_commander;
mod impl_handle_commander_msg;
mod impl_handle_ui;

use std::fmt::Debug;
use std::sync::Arc;
use std::time::Instant;

use bytes::BytesMut;
use serde_json::{Value, json};
use tokio::net::UnixStream;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_rustls::TlsConnector;
use tracing::trace;
use zxc_derive::{Buffer, CloseAction, FlushStorage, NotifyCommander};

use crate::CAPACITY_2MB;
use crate::async_step::AsyncStep;
use crate::commander::codec::perform_codec_op;
use crate::forward_info::ForwardInfo;
use crate::io::*;
use crate::proxy::*;
use crate::repeater::prelude::*;
use crate::run::boundary::{
    Buffer, CloseAction, FlushStorage, HandleCommander, NotifyCommander,
    UiReply,
};

// Handler for Repeater side of the PROXY
#[derive(Buffer, FlushStorage, CloseAction, NotifyCommander)]
pub struct RepeaterHandler {
    buf: BytesMut,
    tls_connector: Arc<TlsConnector>,
    ws_handles: Vec<RepeaterWsHandle>,
    ws_index: usize,
    from_commander: Receiver<ForwardInfo>,
    to_commander: Sender<ForwardInfo>,
}

impl RepeaterHandler {
    #[inline(always)]
    pub fn new(
        tls_connector: Arc<TlsConnector>,
        from_commander: Receiver<ForwardInfo>,
        to_commander: Sender<ForwardInfo>,
    ) -> RepeaterHandler {
        RepeaterHandler {
            buf: BytesMut::with_capacity(CAPACITY_2MB),
            tls_connector,
            ws_handles: vec![],
            ws_index: 1,
            from_commander,
            to_commander,
        }
    }

    /// Drops handles whose WebSocket task has already exited.
    ///
    /// # Steps
    ///
    /// 1. Keep only handles where `JoinHandle::is_finished()` is `false`.
    ///
    /// The task returns when the server closes the stream or when the task
    /// errors. Neither path tells the UI, so the handle would otherwise stay
    /// in `ws_handles` for the process lifetime.
    fn reap_finished_ws(&mut self) {
        let before = self.ws_handles.len();
        self.ws_handles
            .retain(|ws| !ws.handle.is_finished());
        let reaped = before - self.ws_handles.len();
        if reaped > 0 {
            trace!("ws reaped| {}", reaped);
        }
    }

    /// Runs the initial repeater state machine (`RepeaterConnState`).
    ///
    /// # Steps
    ///
    /// 1. Convert `SendInfo` to `RepeaterConn`.
    /// 2. Create `RepeaterConnState::EstablishServerConn`.
    /// 3. Run `RepeaterConnState`.
    /// 4. Jump to the encryption state when a connector is needed.
    /// 5. Return when the state `is_ended()`.
    ///
    /// # Errors
    ///
    /// `RepeaterError::InvalidAddress` [1].
    pub async fn establish_conn(
        &self,
        info: SendInfo,
    ) -> Result<RepeaterConnState, RepeaterError> {
        trace!(
            "send metadata| is_h2={:?} has_h2_info={}",
            info.is_h2,
            info.h2_info.is_some()
        );
        if info.is_h2 == Some(true) && !info.server_info.is_tls() {
            return Err(RepeaterError::H2RequiresTls);
        }
        let rconn = RepeaterConn::<ZStream>::try_from(info)?;
        let mut state = RepeaterConnState::EstablishServerConn(rconn);
        loop {
            state = state.next().await?;
            // 4. If need encryption then jump to encryption state
            if let RepeaterConnState::NeedConnector(rconn) = state {
                state = RepeaterConnState::EncryptConnection(
                    rconn,
                    self.tls_connector.clone(),
                );
                continue;
            }
            if state.is_ended() {
                return Ok(state);
            }
        }
    }

    pub async fn get_response(
        &mut self,
        msg: RepeaterMsg,
    ) -> Result<Option<String>, RepeaterError> {
        self.reap_finished_ws();
        let output: Option<Value> = match msg.operation {
            // Forwards `info` and returns the resulting reply.
            //
            // `handle_commander_ui()` already returns a `UiReply`, so the
            // result is returned directly.
            Operation::Forward(info) => {
                if let UiReply::Reply(_, result) =
                    self.handle_commander_ui(info).await?
                {
                    return Ok(Some(result));
                }
                None
            }
            // Encodes `data` with `codec`.
            //
            // # Errors
            //
            // `RepeaterError::Codec`.
            Operation::Encode {
                codec,
                data,
            } => {
                let result = perform_codec_op(true, &codec, data.as_bytes())?;
                Some(result)
            }

            // Decodes `data` with `codec`.
            //
            // # Errors
            //
            // `RepeaterError::Codec`.
            Operation::Decode {
                codec,
                data,
            } => {
                let result = perform_codec_op(false, &codec, data.as_bytes())?;
                Some(result)
            }

            // Sends a request and measures the response.
            //
            // # Steps
            //
            // 1. Start the timer.
            // 2. Start the initial `RepeaterState` machine.
            // 3. Call `handle_http()` for `HandleTcp` or `HandleTls`, and
            //    `handle_h2()` for `HandleH2`, then get the payload.
            // 4. Calculate the response length.
            // 5. Stop the timer and calculate the response time.
            // 6. Build `{"size": size, "time": time}`.
            //
            // # Errors
            //
            // `RepeaterError`.
            Operation::Send(info) => {
                let start_time = Instant::now();

                let response_data = match self.establish_conn(info).await? {
                    RepeaterConnState::HandleTcp(conn) => {
                        let mut hconn = handle_http(conn).await?;
                        hconn.get_payload()
                    }
                    RepeaterConnState::HandleTls(conn) => {
                        let mut hconn = handle_http(conn).await?;
                        hconn.get_payload()
                    }
                    RepeaterConnState::HandleH2(conn) => {
                        let mut hconn = handle_h2(conn).await?;
                        hconn.get_payload()
                    }

                    _ => unreachable!(),
                };

                let size = response_data
                    .as_ref()
                    .map(|data| data.len())
                    .unwrap_or(0);

                let end_time = Instant::now();

                let time = end_time
                    .duration_since(start_time)
                    .as_millis();

                trace!("size| {}", size);
                trace!("time| {}", time);

                Some(json!({"size" : size, "time": time}))
            }

            // Establishes a WebSocket repeater session.
            //
            // # Steps
            //
            // 1. Call `establish_conn()`.
            // 2. Call `spawn_repeater_ws()` for `HandleTcp` or `HandleTls`
            //    and get a `RepeaterWsHandle`.
            // 3. `HandleH2` returns `RepeaterError::H2WsUnsupported`.
            // 4. Build `{"id": self.ws_index}`.
            // 5. Push the handle to `self.ws_handles` and increment
            //    `self.ws_index`.
            //
            // # Errors
            //
            // `RepeaterError::H2WsUnsupported`.
            Operation::WsEstablish(info) => {
                let handle = match self.establish_conn(info).await? {
                    RepeaterConnState::HandleTcp(conn) => {
                        spawn_repeater_ws(conn, self.ws_index).await
                    }
                    RepeaterConnState::HandleTls(conn) => {
                        spawn_repeater_ws(conn, self.ws_index).await
                    }
                    RepeaterConnState::HandleH2(_) => {
                        return Err(RepeaterError::H2WsUnsupported);
                    }
                    _ => unreachable!(),
                }?;

                self.ws_handles.push(handle);
                let output = Some(json!({"id": self.ws_index}));
                self.ws_index += 1;
                output
            }

            // Sends a WebSocket frame to session `id`.
            //
            // `is_bin` selects the binary opcode.
            //
            // # Errors
            //
            // - `RepeaterError::WsIdNotFound`
            // - `RepeaterWsMessage::Read`.
            Operation::WsSend { id, is_bin } => {
                trace!("ws send| {}", id);
                self.ws_handles
                    .iter_mut()
                    .find(|handle| handle.id == id)
                    .ok_or(RepeaterError::WsIdNotFound(id))?
                    .send(RepeaterWsMsg::Read { is_bin })
                    .await?;
                None
            }

            // Closes WebSocket session `id`.
            //
            // The task may already be gone after a server close or task
            // error, so closing an unknown ID is a no-op.
            Operation::WsClose(id) => {
                let index = self
                    .ws_handles
                    .iter()
                    .position(|handler| handler.id == id);
                if let Some(index) = index {
                    let handler = self.ws_handles.swap_remove(index);
                    let _ = handler.send(RepeaterWsMsg::Close).await;
                    let _ = handler.handle.await;
                    trace!("ws closed| {}", id);
                }
                None
            }
            Operation::Close => {
                trace!("ui close");
                return Err(UnixSockError::Closed.into());
            }
        };
        Ok(output.map(|s| s.to_string()))
    }
}

impl Debug for RepeaterHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "repeater")
    }
}
