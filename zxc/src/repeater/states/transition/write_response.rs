use std::path::PathBuf;

use tracing::trace;

use crate::io::*;

/// Logs the response to a file.
pub trait WriteResponse {
    fn update_response_path(&mut self);

    fn response_path(&self) -> &PathBuf;

    fn response_data(&self) -> &[u8];
}

/// Logs the response to disk.
pub async fn log_response<T>(mut conn: T) -> Result<T, FileErrorInfo>
where
    T: WriteResponse,
{
    conn.update_response_path();
    let path = conn.response_path();
    trace!("{:?}", path);
    let response_data = conn.response_data();
    if !response_data.is_empty() {
        create_and_write_file(path, conn.response_data()).await?;
    }
    Ok(conn)
}
