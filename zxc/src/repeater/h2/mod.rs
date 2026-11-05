mod pipeline;

use std::path::PathBuf;

use bytes::BytesMut;
use http_plz::Request;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use two_plz::client::{ClientConnection, ResponseFuture};
use zxc_derive::RepeaterReadFile;

use crate::async_step::async_run;
use crate::forward_info::H2Info;
use crate::io::*;
use crate::proxy::*;
use crate::repeater::prelude::*;

// repeater h2 handler struct
#[derive(RepeaterReadFile)]
pub struct Rh2<T> {
    buf: BytesMut,
    path: PathBuf,
    stream: Option<T>,
    file: File,
    payload: Option<BytesMut>,
    update: bool,
    h2_info: Option<H2Info>,
    pending_request: Option<Request>,
    client_conn: Option<ClientConnection<T>>,
    response_future: Option<ResponseFuture>,
}

impl<T> Rh2<T> {
    pub fn get_payload(&mut self) -> Option<BytesMut> {
        self.payload.take()
    }
}

// One request/response per connection. The H2 connection driver and the
// response future are polled directly by the repeater state machine.
pub async fn handle_h2<T>(
    conn: RepeaterConn<T>,
) -> Result<Rh2<T>, ProxyStateError>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
{
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&conn.path)
        .await
        .map_err(|e| FileErrorInfo::from((&conn.path, FileEvent::Open, e)))?;
    let h2: Rh2<T> = Rh2::from((conn, file));
    let state = RepeaterState::ReadFromFile(h2);
    match async_run(state).await? {
        RepeaterState::End(hconn) => Ok(hconn),
        _ => {
            unreachable!();
        }
    }
}
