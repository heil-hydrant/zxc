use tracing::trace;

use super::should_update::ShouldUpdate;
use crate::io::*;
use crate::proxy::*;
use crate::repeater::prelude::*;

/// Reads the payload from disk and applies the update policy.
///
/// # Steps
///
/// 1. Read from the file.
/// 2. Transition to `UpdateFrame` when `should_update` is `true`. This
///    is always true for WebSocket and the default for HTTP.
/// 3. Otherwise add the raw data and transition to `Send`. HTTP only,
///    when the user sets `b:update` to `false` in the UI.
///
/// # Transitions
///
/// `ReadFromFile` -> `UpdateFrame` | `Send`.
///
/// # Errors
///
/// - `ProtocolError::File` [1]
/// - `ProtocolError::UpdateFrame` [3]
pub async fn read_from_file<T>(
    mut conn: T,
) -> Result<RepeaterState<T>, ProtocolError>
where
    T: FileBuf + ShouldUpdate + AddRaw,
{
    let (file, buf) = conn.file_and_buf_as_mut();
    read_file(file, buf).await?;
    let nbuf = buf.split();
    if conn.should_update() {
        trace!("should_update| Y");
        return Ok(RepeaterState::UpdateFrame(conn, nbuf));
    }
    trace!("should_update| N");
    conn.add_raw(nbuf)?;
    Ok(RepeaterState::Send(conn))
}
