use bytes::BytesMut;
use thiserror::Error;
use tracing::trace;

use super::msg_to_bytes::MsgToBytes;
use crate::interceptor::*;
use crate::proxy::prelude::*;

// Trait to parse bytes to msg.
pub trait BytesToMsg {
    fn parse_bytes(&mut self, buf: BytesMut) -> Result<(), UpdateMsgError>;
}

// Trait to check if http request/response should be rewritten.
// http only.
//
// For http,
//      If user sets b:update to true , i.e. frame should be updated according
//      to rfc, then request/response should be rewritten.
//
//      If b:update is false, since the file is already written by UI, no need
//      to rewrite.
//
// For ws, already rewritten by ui and no additional data is added. So,
// always false.

pub trait ShouldRewrite {
    fn should_rewrite(&self, resume_info: &ResumeInfo) -> bool;
}

#[derive(Error, Debug)]
pub enum UpdateMsgError {
    #[error("updating frame")]
    HttpFrame(#[from] http_plz::MsgParseErr),
    #[error("invalid ws frame")]
    InvalidWsFrame,
    #[error("invalid uri")]
    InvalidUri(#[from] header_plz::uri::InvalidUri),
    #[error("invalid status code")]
    InvalidStatusCode(#[from] header_plz::status::InvalidStatusCode),
}

pub fn update_msg<T>(
    mut conn: T,
    buf: BytesMut,
    resume_info: ResumeInfo,
) -> Result<ProxyState<T>, ProxyStateError>
where
    T: BytesToMsg + MsgToBytes + ShouldRewrite,
{
    conn.parse_bytes(buf)?;
    conn.msg_to_bytes();
    if conn.should_rewrite(&resume_info) {
        trace!("rewrite| Y");
        return Ok(ProxyState::ReWrite(conn, resume_info));
    }
    trace!("send");
    Ok(ProxyState::Send(conn))
}
