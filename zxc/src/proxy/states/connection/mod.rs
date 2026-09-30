use std::sync::Arc;

use bytes::BytesMut;
use http_plz::OneRequest;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc::Sender;

use super::{ServerInfo, Tcp};
use crate::CAPACITY_2MB;
use crate::commander::CommanderRequest;
mod convert;
pub mod encrypt;

// Zero sized struct to denote no stream
pub struct ZStream;

pub struct Connection<T, E> {
    pub id: usize,
    pub commander: Sender<CommanderRequest>,
    pub request: Option<OneRequest>,
    pub buf: BytesMut,
    pub reader: T, // client
    pub writer: E, // server
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
