pub mod additional_handler_info;
pub mod error;
pub mod handlers;
pub mod read_write;
pub mod role;
pub mod transition;

use std::fmt::Display;

use bytes::BytesMut;
pub use error::ProxyStateError;
pub use read_write::ReadWrite;
pub use role::GetRole;
pub use transition::*;

use crate::async_step::AsyncStep;
use crate::commander::*;
use crate::interceptor::*;
use crate::proxy::prelude::*;

// All possible Handler States
// In order
pub enum ProxyState<T> {
    Receive(T),
    // log
    ShouldLog(T),
    WriteHistory(T),
    Log(T),
    // intercept
    ShouldIntercept(T),
    Intercept(T),
    // resume
    ResumeIntercept(T),
    ReadModFile(T, ResumeInfo),
    UpdateMsg(T, BytesMut, ResumeInfo),
    ReWrite(T, ResumeInfo),
    NewConnection(T, ServerInfoJson), // http/1 reconnect; h2 currently ignored
    Send(T),
    Drop(T),
    End(T),
    SwitchProtocol(T, Protocol),     // http only
    ServerClose(T, ProxyStateError), // http only
}

impl<T> AsyncStep for ProxyState<T>
where
    Self: Sized,
    T: AddRaw
        + BytesToMsg
        + CanCommunicate
        + CanLog
        + DropMsg
        + FileOps
        + GetHistory
        + GetRole
        + Id
        + Intercept
        + Log
        + MsgToBytes
        + ReadWrite<State = Self>
        + Rewrite
        + SendHistory
        + ShouldIntercept
        + ShouldLog
        + ShouldRewrite
        + UpdateLogExt
        + UpdateResumeInfo,
    <T as ReadWrite>::Error: Into<ProxyStateError>,
{
    type Error = ProxyStateError;

    async fn next(self) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        match self {
            // 1. Read -> ShouldLog | ServerClose
            Self::Receive(conn) => conn.read().await.map_err(Into::into),
            // 2. ShouldLog -> WriteHistory | Send
            Self::ShouldLog(conn) => should_log(conn).await,
            // 3. WriteHistory -> Log
            Self::WriteHistory(conn) => write_history(conn).await,
            // 4. Log -> ShouldIntercept
            Self::Log(conn) => write_log(conn).await,
            // 5. ShouldIntercept -> Intercept | Send
            Self::ShouldIntercept(conn) => should_intercept(conn).await,
            // 6. Intercept -> ResumeIntercept | Drop
            Self::Intercept(conn) => intercept(conn).await,
            // 7. Drop -> End
            Self::Drop(conn) => {
                T::continue_on_drop()?;
                Ok(Self::Receive(conn))
            }
            // 8. ResumeIntercept -> ReadModFile | NewConnection | Send
            Self::ResumeIntercept(conn) => resume_intercept(conn).await,
            // 9. ReadModFile -> UpdateFrame | Send
            Self::ReadModFile(conn, resume_info) => {
                read_mod_file(conn, resume_info).await
            }
            // 10. UpdateFrame -> ReWrite | Send
            Self::UpdateMsg(conn, buf, resume_info) => {
                update_msg(conn, buf, resume_info)
            }
            // 11. ReWrite -> Send
            Self::ReWrite(conn, resume_info) => {
                rewrite_log(conn, resume_info).await
            }
            // 12. Send -> End | ReadFail | Read (full duplex)
            Self::Send(conn) => conn.write().await.map_err(Into::into),
            _ => Ok(self),
        }
    }

    fn is_ended(&self) -> bool {
        matches!(self, Self::End(_))
            || matches!(self, Self::ServerClose(_, _))
            || matches!(self, Self::NewConnection(_, _))
    }
}

impl<T> Display for ProxyState<T>
where
    T: GetRole,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (conn, val) = match self {
            Self::Receive(conn) => (conn, "receive"),
            Self::ShouldLog(conn) => (conn, "should_log"),
            Self::WriteHistory(conn) => (conn, "write_history"),
            Self::Log(conn) => (conn, "log"),
            Self::ShouldIntercept(conn) => (conn, "should_intercept"),
            Self::Intercept(conn) => (conn, "intercept"),
            Self::ResumeIntercept(conn) => (conn, "resume_intercept"),
            Self::ReadModFile(conn, _) => (conn, "read_mod_file"),
            Self::UpdateMsg(conn, ..) => (conn, "update_frame"),
            Self::ReWrite(conn, _) => (conn, "rewrite"),
            Self::Send(conn) => (conn, "send"),
            Self::SwitchProtocol(conn, _) => (conn, "switch_protocol"),
            _ => return Err(std::fmt::Error),
        };
        write!(f, "[{}::{}]", conn.role(), val)
    }
}
