use std::io::SeekFrom;

use bytes::BytesMut;
use tokio::io::AsyncSeekExt;
use tracing::trace;

use crate::interceptor::*;
use crate::io::file::{FileErrorInfo, read_file};
use crate::proxy::prelude::*;

pub const WS_ADD_RAW_PANIC: &str = "No add raw for ws";

// Trait to add bytes to data without any protocol checks.
//
// http frames can/cannot be updated based on b:update value set by user in ui.
//
// ws frames are always updated. (Blanket implementation)

pub trait AddRaw {
    fn add_raw(&mut self, buf: BytesMut) -> Result<(), UpdateMsgError>;
}

pub async fn read_mod_file<T>(
    mut conn: T,
    resume_info: ResumeInfo,
) -> Result<ProxyState<T>, ProxyStateError>
where
    T: super::AttachFile + AddRaw,
{
    let (file, buf) = conn.file_and_buf_as_mut();
    if let Err(e) = file.seek(SeekFrom::Start(0)).await {
        return Err(FileErrorInfo::seek_fail(conn.file_path(), e).into());
    }
    if let Err(e) = read_file(file, buf).await {
        return Err(FileErrorInfo::read_fail(conn.file_path(), e).into());
    }
    let data = buf.split();
    if resume_info.update() {
        trace!("update| Y");
        return Ok(ProxyState::UpdateMsg(conn, data, resume_info));
    } else {
        trace!("update| N");
        conn.add_raw(data)?;
    }

    if let Some(server_info) = resume_info.into_server_info() {
        trace!("new conn");
        return Ok(ProxyState::NewConnection(conn, server_info));
    }
    trace!("send");
    Ok(ProxyState::Send(conn))
}
