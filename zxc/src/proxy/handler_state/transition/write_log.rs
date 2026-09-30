use bytes::Buf;
use bytes::BytesMut;
use std::path::PathBuf;
use tokio::fs::File;
use tracing::trace;

use super::frame_to_payload::FrameToPayload;
use crate::io::file::create_and_write_file;
use crate::proxy::handler_state::{ProxyState, ProxyStateError};

// Trait to perform file operations
//
// Implemented In: (derive macro)
//     zxc-derive/src/file_ops/mod.rs

pub trait FileOps {
    fn file_and_buf_as_mut(&mut self) -> (&mut File, &mut BytesMut);

    fn attach_file(&mut self, file: File);
}

// Trait to update the extension of the request/response.
pub trait UpdateLogExt {
    fn update_extension(&mut self);
}

pub trait Log {
    fn path(&self) -> &PathBuf;

    fn log_data(&self) -> &[u8];
}

// Description:
//      Transition function to write the http/ws request/response to a file.
//
// Transition:
//      WriteLog -> ShouldIntercept
//
// Steps:
//      1. Update path to write to.
//      2. Convert frame to payload. http only
//      3. Create and Write file for the path
//      4. Attach file, can be used in resume_intercept.
//
// Error:
//      ProxyStateError::FileIo  [5]

pub async fn write_log<T>(
    mut conn: T,
) -> Result<ProxyState<T>, ProxyStateError>
where
    T: UpdateLogExt + Log + FileOps + FrameToPayload,
{
    conn.update_extension();
    conn.frame_to_payload();
    let file = create_and_write_file(conn.path(), conn.log_data()).await?;
    conn.attach_file(file);
    trace!("[+] message logged| {}", conn.path().display());
    Ok(ProxyState::ShouldIntercept(conn))
}
