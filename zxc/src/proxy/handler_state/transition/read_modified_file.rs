use std::io::SeekFrom;

use bytes::BytesMut;
use tokio::io::AsyncSeekExt;
use tracing::trace;

use crate::interceptor::message::from_ui::resume_info::ResumeInfo;
use crate::io::file::{FileErrorInfo, FileEvent, read_file};
use crate::proxy::handler_state::ProxyState;
use crate::proxy::handler_state::error::ProxyStateError;

pub const WS_ADD_RAW_PANIC: &str = "No add raw for ws";

// Trait to add bytes to data without any protocol checks.
//
// http frames can/cannot be updated based on b:update value set by user in ui.
//
// ws frames are always updated. (Blanket implementation)

pub trait AddRaw {
    fn add_raw(&mut self, buf: BytesMut);
}

// Description:
//      Transition Function for read modified file.
//
// Transition:
//      ReadModFile -> UpdateFrame | NewConnection | Send
//
// Steps:
//      1. Get mutable reference to file and buf.
//      2. Seek to start.
//      3. Read file.
//      4. Split filled buf.
//      5. If update(),
//          a. For http, b:update set by user in ui
//          b. For ws, always true
//                  return ProxyState::UpdateFrame
//
//      6. Else, add_raw payload (http only)
//      7. Else if, resume_info.into_server_info() is some (http req) only
//          return ProxyState::NewConnection
//      8. Default, ProxyState::Send
//
// Error:
//      ProxyStateError::FileIo   [2] [3]

pub async fn read_mod_file<T>(
    mut conn: T,
    resume_info: ResumeInfo,
) -> Result<ProxyState<T>, ProxyStateError>
where
    T: super::FileOps + AddRaw,
{
    let (file, buf) = conn.file_and_buf_as_mut();
    if let Err(e) = file.seek(SeekFrom::Start(0)).await {
        return Err(FileErrorInfo::from((file, FileEvent::Seek, e)).into());
    }
    read_file(file, buf).await?;
    let fbuf = buf.split();
    if resume_info.update() {
        trace!("update| Y");
        return Ok(ProxyState::UpdateFrame(conn, fbuf, resume_info));
    } else {
        trace!("update| N");
        conn.add_raw(fbuf);
    }

    if let Some(server_info) = resume_info.into_server_info() {
        trace!("new conn");
        return Ok(ProxyState::NewConnection(conn, server_info));
    }
    // 6. else
    trace!("send");
    Ok(ProxyState::Send(conn))
}
