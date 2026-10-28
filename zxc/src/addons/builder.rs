use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use tokio::fs::create_dir;
use tracing::trace;

use crate::ADDONS;
use crate::file_types::EXT_REQ;
use crate::io::{DirError, incremental};

/* Steps:
 *      1. Check if the path has enough components, i.e. 3
 *              ./history/1/1.req
 *
 *      2. Pop (component_count - 3) times
 *              ./history/1
 *
 *      3. Push addons to the path and create_dir, if Error is AlreadyExists
 *      do nothing.
 *              ./history/1/addons
 *
 *      4. Get Incremental file for the addon prefix
 *
 *      5. Push prefix and incremental to the path
 *              ./history/1/addons/$prefix-$incremental.req
 *
 * Error:
 *      DirError::NoTop             [1]
 *      DirError::CreateDir         [3]
 *      DirError::GetIncrementalDir [4]
 */

pub async fn build_addon_dest(
    prefix: &str,
    src_file: &Path,
) -> Result<PathBuf, DirError> {
    let mut path = src_file.to_path_buf();

    if path.components().count() < 3 {
        return Err(DirError::NoTop(src_file.to_string_lossy().to_string()));
    }
    for _ in 0..(path.components().count() - 3) {
        path.pop();
    }
    trace!("top| {}", &path.to_string_lossy());
    path.push(ADDONS);
    if let Err(e) = create_dir(&path).await
        && !matches!(e.kind(), ErrorKind::AlreadyExists)
    {
        return Err(DirError::CreateDir(e));
    }
    let incremental = incremental(&path, prefix, false)
        .await
        .map_err(DirError::IncrementalDir)?;
    path.push(format!("{}{}.{}", prefix, incremental, EXT_REQ));
    Ok(path)
}

#[cfg(test)]
pub mod tests {
    use std::env::set_current_dir;
    use std::error::Error;
    use std::path::PathBuf;

    use super::*;

    #[tokio::test]
    async fn test_zz_incremental_file_last() -> Result<(), Box<dyn Error>> {
        let test_dir = PathBuf::from("/tmp/zxc_test");
        set_current_dir(&test_dir)?;

        let prefixes = [("q-", "sqlmap"), ("z-", "ffuf")];
        let mut path = PathBuf::from("./history/");

        for i in 1..11 {
            path.push(i.to_string());
            path.push(i.to_string());
            path.set_extension("req");

            for (prefix, name) in &prefixes {
                let result = build_addon_dest(prefix, &path).await?;
                let verify = PathBuf::from(format!(
                    "./history/{}/addons/{}{}.req",
                    i,
                    prefix,
                    i + 1
                ));
                assert_eq!(
                    result, verify,
                    "{} test failed for index {}",
                    name, i
                );
            }

            path.pop();
            path.pop();
        }
        Ok(())
    }
}
