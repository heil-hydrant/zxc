use std::fmt::Debug;

use header_plz::status::InvalidStatusCode;
use header_plz::{InfoLineError, OneRequestLine, OneResponseLine};
use thiserror::Error;

use super::OneOneConn;
use crate::proxy::prelude::*;

// one one errors

#[derive(Error, Debug)]
pub enum HandleOneOneError<T, E> {
    // client/server state
    #[error(transparent)]
    ProxyError(#[from] ProxyStateError),
    // ----- client state => client conn
    #[error("server send: {1}")]
    SendToServer(OneOneConn<T, E, OneRequestLine>, #[source] ProxyStateError),
    #[error("new connection required")]
    NeedNewConnection(OneOneConn<T, E, OneRequestLine>, ServerInfoJson),
    // server state
    #[error("server read: {1}")]
    ReadFromServer(
        OneOneConn<E, T, OneResponseLine>,
        #[source] ProxyStateError,
    ),
    // ws upgrade
    #[error(transparent)]
    InfoLine(#[from] InfoLineError),
    #[error(transparent)]
    StatusCode(#[from] InvalidStatusCode),
}
