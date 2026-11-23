use std::path::{Path, PathBuf};

use tokio::fs::File;
use tracing::trace;

use crate::io::*;

/// Persists the payload to disk.
///
/// Either rewrites the existing file or writes a new one depending on
/// `should_rewrite`.
///
/// # Transitions
///
/// `ReWrite` -> `Send`.
///
/// # Note
///
/// HTTP implementations rewrite the existing file. WebSocket
/// implementations write a new file and update the path.
pub trait Persist {
    fn should_rewrite(&self) -> bool;

    fn file_data_and_path(&mut self) -> (&mut File, &[u8], &Path);

    fn update_path(&mut self);

    fn data_as_ref(&self) -> &[u8];

    fn path_as_ref(&self) -> &PathBuf;
}

/// Persists the payload to disk.
///
/// # Steps
///
/// 1. Check `should_rewrite`:
///   - HTTP rewrites when the frame was updated, that is, when the user
///     set `b:update` in the UI.
///   - WebSocket always returns `false`.
/// 2. When rewriting, get the file, data, and path and call `rewrite_file`.
/// 3. When writing a new file:
///   - Update the path.
///   - Call `create_and_write_file`.
///
/// # Transitions
///
/// `ReWrite` -> `Send`.
///
/// # Errors
///
/// `FileErrorInfo` [2] [3].
pub async fn rewrite<T>(mut conn: T) -> Result<T, FileErrorInfo>
where
    T: Persist,
{
    // 1
    if conn.should_rewrite() {
        // 2
        let (file, data, path) = conn.file_data_and_path();
        trace!("rewrite| {:?}", file);
        rewrite_file(file, data, path).await?;

        // 3
    } else {
        conn.update_path();
        trace!("newrite| {:?}", conn.path_as_ref());
        let _ = create_and_write_file(conn.path_as_ref(), conn.data_as_ref())
            .await?;
    }
    Ok(conn)
}
