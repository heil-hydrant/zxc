use tracing::trace;

use super::frame_to_payload::FrameToPayload;
use crate::io::file::create_and_write_file;
use crate::proxy::handler_state::{ProxyState, ProxyStateError};
pub mod log;
use log::Log;
pub mod file_ops;
use file_ops::FileOps;
pub mod update_log_extension;
use update_log_extension::UpdateLogExt;

/* Description:
 *      Transition function to write the http/ws request/response to a file.
 *
 * Transition:
 *      WriteLog -> ShouldIntercept
 *
 * Steps:
 *      1. Update path to write to.
 *      2. Convert frame to payload. http only
 *      3. Create and Write file for the path
 *      4. Attach file, can be used in resume_intercept.
 *
 * Error:
 *      ProxyStateError::FileIo  [5]
 */

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
