use std::path::PathBuf;

use tracing::trace;

use crate::io::*;

/* Description:
 *     Trait to log response to file.
 *
 * Transition:
 *     WriteResponse -> End
 */

pub trait WriteResponse {
    fn update_response_path(&mut self);
    fn response_path(&self) -> &PathBuf;
    fn response_data(&self) -> &[u8];
}

/* Description:
 *      Function to log response to file.
 *
 * Steps:
 *     1. Update the response path
 *     2. Get the path
 *     3. Create the file and Write data
 *
 * Transition:
 *     WriteResponse -> End
 *
 * Error:
 *     std::io::Error
 */

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
