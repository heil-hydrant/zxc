use std::path::PathBuf;

use tokio::fs::{copy, create_dir};
use tracing::trace;

use crate::file_types::{EXT_REQ, EXT_WREQ};
use crate::io::*;

pub const REPEATER_HTTP_FILENAME_REQ: &str = "rep.req";
pub const REPEATER_HTTP_FILENAME_RES: &str = "rep.res";
pub const REPEATER_WS_SCRATCH_FILENAME: &str = "scratch.wreq";

pub const REPEATER_HTTP_BUILD_INFO: (usize, &str, &str) =
    (3, "r-", REPEATER_HTTP_FILENAME_REQ);

pub const REPEATER_WS_BUILD_INFO: (usize, &str, &str) =
    (4, "r-ws-", REPEATER_WS_SCRATCH_FILENAME);

/* Steps:
 *      1. Based on the given extension, select build info
 *              a. threshold
 *              b. prefix
 *              c. dest_filename
 *
 *      2. Check if path has enough components, i.e. < threshold
 *
 *              ./history/1/1.req
 *              ./history/1/websocket/1.wreq
 *
 *      3. Pop path components until (components - threshold) to get top dir
 *              ./history/1
 *
 *      4. Get next incremental index, for selected prefix by calling
 *         incremental()
 *
 *      5. Build destination dir, by pushing prefix and incremental index
 *              ./history/1/r-1
 *              ./history/1/websocket/r-ws-1
 *
 *      6. Create dir
 *
 *      7. Build destination file, by pushing selected dest_filename
 *              ./history/1/r-1/rep.req
 *              ./history/1/websocket/r-ws-1/scratch.wreq
 *
 *      8. Copy src_file to destination
 *
 * Error:
 *      DirError
 *          UnknownExt          [1]
 *          NoTop               [2]
 *          IncrementalDir      [4]
 *          CreateDir           [6]
 *          CopyFile            [8]
 *
 */
pub async fn build_repeater_dest(
    src_file: &PathBuf,
    extension: &str,
) -> Result<PathBuf, DirError> {
    let mut path = src_file.clone();
    let (threshold, prefix, dest_filename) = match extension {
        EXT_REQ => REPEATER_HTTP_BUILD_INFO,
        EXT_WREQ => REPEATER_WS_BUILD_INFO,
        _ => {
            return Err(DirError::UnknownExt(
                src_file.to_string_lossy().to_string(),
            ));
        }
    };

    if path.components().count() < threshold {
        return Err(DirError::NoTop(src_file.to_string_lossy().to_string()));
    }

    for _ in 0..(path.components().count() - threshold) {
        path.pop();
    }

    trace!("top| {}", &path.to_string_lossy());

    let next = incremental(&path, prefix, true)
        .await
        .map_err(DirError::IncrementalDir)?;

    path.push(format!("{}{}", prefix, next));
    trace!("top_dir| {}", &path.to_string_lossy());

    create_dir(&path)
        .await
        .map_err(DirError::CreateDir)?;

    path.push(dest_filename);
    trace!("dest_file| {}", &path.to_string_lossy());

    // copy src to dest_file
    copy(&src_file, &path)
        .await
        .map_err(DirError::CopyFile)?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use std::env::set_current_dir;
    use std::error::Error;
    use std::path::PathBuf;

    use super::*;

    #[tokio::test]
    async fn build_http_dir_test() -> Result<(), Box<dyn Error>> {
        let path = PathBuf::from("/tmp/zxc_test");
        set_current_dir(&path)?;

        for i in 1..10 {
            let path = PathBuf::from(format!("./history/{i}/{i}.req"));
            let dest = build_repeater_dest(&path, "req").await?;
            let verify =
                PathBuf::from(format!("./history/{i}/r-{}/rep.req", i + 1));
            assert_eq!(&verify, &dest);
        }

        Ok(())
    }

    #[tokio::test]
    async fn build_ws_dir_test() -> Result<(), Box<dyn Error>> {
        let path = PathBuf::from("/tmp/zxc_test");
        set_current_dir(&path)?;

        for i in 1..10 {
            let path =
                PathBuf::from(format!("./history/{i}/websocket/{i}.wreq"));
            let dest = build_repeater_dest(&path, "wreq").await?;
            let verify = PathBuf::from(format!(
                "./history/{i}/websocket/r-ws-{}/scratch.wreq",
                i + 1
            ));
            assert_eq!(&verify, &dest);
        }

        Ok(())
    }
}
