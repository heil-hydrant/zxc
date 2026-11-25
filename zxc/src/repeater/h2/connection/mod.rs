use std::path::PathBuf;

use bytes::BytesMut;
use http_plz::Request;
use tokio::fs::File;
use two_plz::client::{ClientConnection, ResponseFuture};
use zxc_derive::FileBuf;

use crate::CAPACITY_2MB;
use crate::forward_info::H2Info;
use crate::repeater::prelude::*;

mod pipeline;

// repeater h2 handler struct
#[derive(FileBuf)]
pub(crate) struct Rh2<T> {
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

// Convert (RepeaterConn<T>, File) -> Rh2<T>
impl<T> From<(RepeaterConn<T>, File)> for Rh2<T> {
    fn from((conn, file): (RepeaterConn<T>, File)) -> Self {
        Self {
            buf: BytesMut::with_capacity(CAPACITY_2MB),
            path: conn.path,
            stream: Some(conn.stream),
            file,
            payload: None,
            update: conn.update,
            h2_info: conn.h2_info,
            pending_request: None,
            client_conn: None,
            response_future: None,
        }
    }
}
