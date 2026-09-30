use std::fmt::Debug;

use header_plz::status::InvalidStatusCode;
use header_plz::{InfoLineError, OneRequestLine, OneResponseLine};
use thiserror::Error;

use super::OneOneStruct;
use crate::proxy::handler_state::error::ProxyStateError;
use crate::proxy::server_info::json::ServerInfoJson;

// one one errors

#[derive(Error, Debug)]
pub enum HandleOneOneError<T, E> {
    // client/server state
    #[error("proxy state error| {0}")]
    ProxyError(#[from] ProxyStateError),
    // ----- client state => client conn
    #[error("http read")]
    SendToServer(OneOneStruct<T, E, OneRequestLine>, ProxyStateError),
    #[error("new connection")]
    NeedNewConnection(OneOneStruct<T, E, OneRequestLine>, ServerInfoJson),
    // server state
    #[error("http read")]
    ReadFromServer(OneOneStruct<E, T, OneResponseLine>, ProxyStateError),
    // ws upgrade
    #[error("{0}")]
    InfoLine(#[from] InfoLineError),
    #[error("{0}")]
    StatusCode(#[from] InvalidStatusCode),
}
