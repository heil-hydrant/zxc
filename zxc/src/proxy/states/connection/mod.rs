pub mod encrypt;

use bytes::BytesMut;
use http_plz::OneRequest;
use tokio::sync::mpsc::Sender;

use crate::CAPACITY_2MB;
use crate::commander::*;

// Zero sized struct to denote no stream
pub struct ZStream;

pub struct Connection<T, E> {
    pub(crate) id: usize,
    pub(crate) commander: Sender<CommanderRequest>,
    pub(crate) request: Option<OneRequest>,
    pub(crate) buf: BytesMut,
    pub(crate) reader: T, // client
    pub(crate) writer: E, // server
}

impl<T, E> Connection<T, E> {
    pub fn new(
        index: usize,
        conn: T,
        tx: Sender<CommanderRequest>,
    ) -> Connection<T, ZStream> {
        Connection {
            buf: BytesMut::with_capacity(CAPACITY_2MB),
            commander: tx,
            request: None,
            id: index,
            reader: conn,
            writer: ZStream,
        }
    }
}

// Type-changing struct; see
// https://github.com/rust-lang/rust/issues/86555.

// Establish Server Connection = ZStream -> Tcp
// Complete Handshake = Tls
// TlsTcp = TlsTls -> TlsTcp
// TcpTls = TcpTcp -> TcpTls

impl<T, E, U> From<(Connection<T, E>, U)> for Connection<T, U> {
    fn from((conn, stream): (Connection<T, E>, U)) -> Self {
        Connection {
            writer: stream,
            buf: conn.buf,
            commander: conn.commander,
            request: conn.request,
            id: conn.id,
            reader: conn.reader,
        }
    }
}
