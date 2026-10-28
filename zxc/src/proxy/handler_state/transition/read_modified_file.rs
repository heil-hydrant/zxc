use std::io::SeekFrom;

use bytes::BytesMut;
use tokio::io::AsyncSeekExt;
use tracing::trace;

use crate::interceptor::*;
use crate::io::file::{FileErrorInfo, FileEvent, read_file};
use crate::proxy::prelude::*;

pub const WS_ADD_RAW_PANIC: &str = "No add raw for ws";

// Trait to add bytes to data without any protocol checks.
//
// http frames can/cannot be updated based on b:update value set by user in ui.
//
// ws frames are always updated. (Blanket implementation)

pub trait AddRaw {
    fn add_raw(&mut self, buf: BytesMut);
}

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
        return Ok(ProxyState::UpdateMsg(conn, fbuf, resume_info));
    } else {
        trace!("update| N");
        conn.add_raw(fbuf);
    }

    if let Some(server_info) = resume_info.into_server_info() {
        trace!("new conn");
        return Ok(ProxyState::NewConnection(conn, server_info));
    }
    trace!("send");
    Ok(ProxyState::Send(conn))
}
