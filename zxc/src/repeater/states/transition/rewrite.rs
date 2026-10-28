use std::path::PathBuf;

use tokio::fs::File;
use tracing::trace;

use crate::io::*;

/* Description:
 *      Trait to rewrite file.
 *      Get mutable reference to file and reference to data
 *
 * Transition:
 *     ReWrite -> Send
 *
 * NOTE:
 *      ONLY http implementation is used.
 *      ws implementation to satify trait bound.
 */

pub trait Rewrite {
    fn should_rewrite(&self) -> bool;

    fn get_write_data_and_file(&mut self) -> (&[u8], &mut File);
}

/* Description:
 *      Trait to write new file.
 *
 * Transition:
 *      ReWrite -> Send
 *
 * NOTE:
 *      ONLY WS implementation is used.
 *      http implementation to satify trait bound.
 */

pub trait Newrite {
    fn update_path(&mut self);

    fn data_as_ref(&self) -> &[u8];

    fn path_as_ref(&self) -> &PathBuf;
}

/* Description:
 *      Function to rewrite or newrite file.
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
    T: Rewrite + Newrite,
{
    // 1
    if conn.should_rewrite() {
        // 2
        let (data, file) = conn.get_write_data_and_file();
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
