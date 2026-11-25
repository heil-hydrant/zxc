use tracing::trace;

use super::should_update::ShouldUpdate;
use crate::io::*;
use crate::proxy::*;
use crate::repeater::prelude::*;

/// Reads the payload from disk and applies the update policy.
///
/// # Transitions
///
/// `ReadFromFile` -> `UpdateFrame` | `Send`.
pub async fn read_from_file<T>(
    mut conn: T,
) -> Result<RepeaterState<T>, ProtocolError>
where
    T: FileBuf + ShouldUpdate + AddRaw,
{
    let (file, buf) = conn.file_and_buf_as_mut();
    if let Err(e) = read_file(file, buf).await {
        return Err(FileErrorInfo::read_fail(conn.file_path(), e).into());
    }
    let data = buf.split();
    if conn.should_update() {
        trace!("should_update| Y");
        return Ok(RepeaterState::UpdateFrame(conn, data));
    }
    trace!("should_update| N");
    conn.add_raw(data)?;
    Ok(RepeaterState::Send(conn))
}
