use std::path::PathBuf;

use tokio::fs::File;
use tracing::trace;

use crate::io::*;

/* Description:
 *      Trait to persist the payload to disk.
 *      Either rewrite the existing file or write a new one.
 *
 * Transition:
 *     ReWrite -> Send
 *
 * NOTE:
 *      HTTP implementations rewrite the existing file.
 *      WS implementations write a new file and update the path.
 */

pub trait Persist {
    fn should_rewrite(&self) -> bool;

    fn file_and_data(&mut self) -> (&mut File, &[u8]);

    fn update_path(&mut self);

    fn data_as_ref(&self) -> &[u8];

    fn path_as_ref(&self) -> &PathBuf;
}

/* Description:
 *      Function to persist the payload to disk.
 *
 * Steps:
 *      1. Check if should rewrite,
 *          a. For http, if frame updated then rewrite, i.e. b:update set in UI
 *          b. For ws, always false
 *
 *      2. If rewrite, get data and file and rewrite_file
 *
 *      3. If newrite,
 *          a. update the path
 *          b. create_and_write_file
 *
 * Transition:
 *     ReWrite -> Send
 *
 * Error:
 *     FileErrorInfo [2] [3]
 */

pub async fn rewrite<T>(mut conn: T) -> Result<T, FileErrorInfo>
where
    T: Persist,
{
    // 1
    if conn.should_rewrite() {
        // 2
        let (file, data) = conn.file_and_data();
        trace!("rewrite| {:?}", file);
        rewrite_file(file, data)
            .await
            .map_err(|(event, e)| FileErrorInfo::from((file, event, e)))?;

        // 3
    } else {
        conn.update_path();
        trace!("newrite| {:?}", conn.path_as_ref());
        let _ = create_and_write_file(conn.path_as_ref(), conn.data_as_ref())
            .await?;
    }
    Ok(conn)
}
