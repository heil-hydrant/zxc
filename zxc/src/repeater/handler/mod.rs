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
    Buffer, CloseAction, FlushStorage, HandleCommander, NotifyCommander
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

    /* Description:
     *      Runs the Initial repeater state machine (RepeaterConnState).
     *
     * Steps:
     *      1. Convert SendInfo to RepeaterConn
     *      2. Create RepeaterConnState::EstablishServerConn
     *      3. Run RepeaterConnState
     *      4. If need connector then jump to encryption state
     *      5. If state is_ended() then return
     *
     * Error:
     *      RepeaterError::InvalidAddress   [1]
     */

    pub async fn establish_conn(
        &self,
        info: SendInfo,
    ) -> Result<RepeaterConnState, RepeaterError> {
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
        let output: Option<Value> = match msg.operation {
            /* Associated Values:
             *      info: ForwardInfo
             *
             * Steps:
             *      Since, result from handle_commander_ui() is Option<String>,
             *      return it directly.
             */
            Operation::Forward(info) => {
                if let Some(result) = self.handle_commander_ui(info).await? {
                    return Ok(Some(result));
                }
                None
            }
            /* Associated Values:
             *      codec   : Codec
             *      data    : String
             *
             * Error:
             *      RepeaterError::Codec
             */
            Operation::Encode {
                codec,
                data,
            } => {
                let result = perform_codec_op(true, &codec, data.as_bytes())?;
                Some(result)
            }

            /* Associated Values:
             *      codec   : Codec
             *      data    : String
             *
             * Error:
             *      RepeaterError::Codec
             */
            Operation::Decode {
                codec,
                data,
            } => {
                let result = perform_codec_op(false, &codec, data.as_bytes())?;
                Some(result)
            }

            /* Associated Values:
             *      info    : SendInfo
             *
             * Steps:
             *      1. start timer
             *      2. start initial RepeaterState machine
             *      3. If RepeaterState is HandleTcp/HandleTls call
             *         handle_http(), and get the response.
             *      4. calculate the response length
             *      5. end timer and calculate the response time
             *      6. build output json in format {"size": size, "time": time}
             *
             * Error:
             *      RepeaterError
             */
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

            /* Associated Values:
             *      info    : SendInfo
             *
             * Steps:
             *      1. Call establish_conn()
             *
             *      2. If RepeaterConnState is HandleTcp/HandleTls, call
             *          spawn_repeater_ws() to get RepeaterWsHandle
             *
             *      3. Build output json in format {"id": self.ws_index}
             *
             *      4. Push handle to self.ws_handles and increment
             *         self.ws_index
             */
            Operation::WsEstablish(info) => {
                let handle = match self.establish_conn(info).await? {
                    RepeaterConnState::HandleTcp(conn) => {
                        spawn_repeater_ws(conn, self.ws_index).await
                    }
                    RepeaterConnState::HandleTls(conn) => {
                        spawn_repeater_ws(conn, self.ws_index).await
                    }
                    _ => unreachable!(),
                }?;

                self.ws_handles.push(handle);
                let output = Some(json!({"id": self.ws_index}));
                self.ws_index += 1;
                output
            }

            /* Associated Values:
             *      id  : usize
             *
             * Error:
             *      RepeaterError::WsIdNotFound
             *      RepeaterWsMessage::Read
             */
            Operation::WsSend(id) => {
                trace!("ws send| {}", id);
                self.ws_handles
                    .iter_mut()
                    .find(|handle| handle.id == id)
                    .ok_or(RepeaterError::WsIdNotFound(id))?
                    .send(RepeaterWsMsg::Read)
                    .await?;
                None
            }

            /* Associated Values:
             *      id  : usize
             *
             * Error:
             *      RepeaterError::WsIdNotFound
             *      RepeaterWsMessage::Read
             */
            Operation::WsClose(id) => {
                let index = self
                    .ws_handles
                    .iter()
                    .position(|handler| handler.id == id)
                    .ok_or(RepeaterError::WsIdNotFound(id))?;
                let handler = self.ws_handles.swap_remove(index);
                handler
                    .send(RepeaterWsMsg::Close)
                    .await?;
                let _ = handler.handle.await;
                trace!("ws closed| {}", id);
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
