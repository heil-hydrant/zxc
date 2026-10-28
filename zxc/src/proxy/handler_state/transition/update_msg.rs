use bytes::BytesMut;
use thiserror::Error;
use tracing::trace;

use super::msg_to_bytes::MsgToBytes;
use crate::interceptor::*;
use crate::proxy::prelude::*;

// Trait to check if http request/response should be rewritten.
// http only.
//
// For http,
//      If user sets b:update to true , i.e. frame should be updated according
//      to rfc , then request/response should be rewritten.
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
    #[error("invalid uri")]
    InvalidStatusCode(#[from] header_plz::status::InvalidStatusCode),
}

// Trait to parse bytes to msg.
pub trait BytesToMsg {
    type Msg;

    fn parse_bytes(&self, buf: BytesMut) -> Result<Self::Msg, UpdateMsgError>;

    fn add_msg(&mut self, msg: Self::Msg);

    // Requests logged to disk are stored in origin-form, so parsing them back
    // loses :authority and :scheme. HTTP/2 streams restore them here.
    // No-op for every other handler.
    fn apply_h2_info(
        &mut self,
        _h2_info: Option<&H2Info>,
    ) -> Result<(), UpdateMsgError> {
        Ok(())
    }
}

pub fn update_msg<T>(
    mut conn: T,
    buf: BytesMut,
    resume_info: ResumeInfo,
) -> Result<ProxyState<T>, ProxyStateError>
where
    T: BytesToMsg + MsgToBytes + ShouldRewrite,
{
    let msg = conn.parse_bytes(buf)?;
    conn.add_msg(msg);
    conn.apply_h2_info(resume_info.h2_info())?;
    conn.msg_to_bytes();
    if conn.should_rewrite(&resume_info) {
        trace!("rewrite| Y");
        return Ok(ProxyState::ReWrite(conn, resume_info));
    }
    trace!("send");
    Ok(ProxyState::Send(conn))
}
