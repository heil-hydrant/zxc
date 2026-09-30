use bytes::BytesMut;
use tokio::fs::File;
use tracing::trace;

use super::should_update::ShouldUpdate;
use crate::io::file::{FileErrorInfo, read_file};
use crate::proxy::handler_state::transition::read_modified_file::AddRaw;
use crate::repeater::states::rstate::RepeaterState;

/* Description:
 *     Trait to read from file.
 *
 * Implemented in (as derive macro):
 *     zxc-derive/src/repeater_read_from_file.rs
 */

pub trait RepeaterReadFile {
    fn file_and_buf_as_mut(&mut self) -> (&mut File, &mut BytesMut);
}

/* Steps:
 *     1. Read from the file
 *     2. If should_update is true, transition to UpdateFrame [ Always true for
 *     ws and by default true for http ]
 *     3. else, add as raw data and transition to Send [ http only, when user
 *        sets b:update to false in UI ]
 *
 * Transition:
 *     ReadFromFile -> UpdateFrame | Send
 *
 * Error:
 *      FileErrorInfo   [1]
 */

pub async fn read_from_file<T>(
    mut conn: T,
) -> Result<RepeaterState<T>, FileErrorInfo>
where
    T: RepeaterReadFile + ShouldUpdate + AddRaw,
{
    let (file, buf) = conn.file_and_buf_as_mut();
    read_file(file, buf).await?;
    let nbuf = buf.split();
    if conn.should_update() {
        trace!("should_update| Y");
        return Ok(RepeaterState::UpdateFrame(conn, nbuf));
    }
    trace!("should_update| N");
    conn.add_raw(nbuf);
    Ok(RepeaterState::Send(conn))
}
