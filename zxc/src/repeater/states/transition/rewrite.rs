use std::path::{Path, PathBuf};

use tokio::fs::File;
use tracing::trace;

use crate::io::*;

pub const NEWRITE_HTTP_ERROR: &str = "No new Write for http";

/// Persists the payload to disk.
///
/// Either rewrites the existing file or writes a new one depending on
/// `should_rewrite`.
///
pub trait Persist {
    fn should_rewrite(&self) -> bool;

    fn file_data_and_path(&mut self) -> (&mut File, &[u8], &Path);

    fn update_path(&mut self) {
        panic!("{}", NEWRITE_HTTP_ERROR)
    }

    fn data_as_ref(&self) -> &[u8] {
        panic!("{}", NEWRITE_HTTP_ERROR)
    }

    fn path_as_ref(&self) -> &PathBuf {
        panic!("{}", NEWRITE_HTTP_ERROR)
    }
}

/// Persists the payload to disk.
///
/// # Note
///
/// HTTP implementations rewrite the existing file. WebSocket
/// implementations write a new file and update the path.
pub async fn rewrite<T>(mut conn: T) -> Result<T, FileErrorInfo>
where
    T: Persist,
{
    if conn.should_rewrite() {
        let (file, data, path) = conn.file_data_and_path();
        trace!("rewrite| {:?}", file);
        rewrite_file(file, data, path).await?;
    } else {
        conn.update_path();
        trace!("newrite| {:?}", conn.path_as_ref());
        let _ = create_and_write_file(conn.path_as_ref(), conn.data_as_ref())
            .await?;
    }
    Ok(conn)
}
