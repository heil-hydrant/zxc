use std::io::{self, Error};
use std::path::Path;

use thiserror::Error;
use tokio::fs::read_dir;

#[derive(Debug, Error)]
pub enum DirError {
    #[error("unknown file extension: {0}")]
    UnknownExt(String),
    #[error("top directory not found: {0}")]
    NoTop(String),
    #[error("incremental directory build: {0}")]
    IncrementalDir(#[source] io::Error),
    #[error("directory create: {0}")]
    CreateDir(#[source] io::Error),
    #[error("file copy: {0}")]
    CopyFile(#[source] io::Error),
}

pub async fn incremental<T>(
    directory: T,
    prefix: &str,
    for_dir: bool,
) -> Result<usize, Error>
where
    T: AsRef<Path>,
{
    let mut id = 0;
    let mut dir_list = read_dir(directory).await?;

    while let Some(entry) = dir_list.next_entry().await? {
        let name = entry.file_name();

        let dir_str = if for_dir && entry.file_type().await?.is_dir() {
            name
        } else if entry.file_type().await?.is_file() {
            let path: &Path = name.as_ref();
            path.file_stem().unwrap().into()
        } else {
            continue;
        };
        let high: usize = dir_str
            .to_str()
            .and_then(|s| s.split(prefix).nth(1))
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        if high > id {
            id = high;
        }
    }

    id += 1;
    Ok(id)
}

#[cfg(test)]
pub mod tests {

    use std::path::PathBuf;

    use super::*;

    #[tokio::test]
    async fn test_incremental_dir() -> Result<(), Box<dyn std::error::Error>> {
        let prefix = "r-";
        let mut path = PathBuf::from("/tmp/zxc_test/history/");
        for i in 1..11 {
            path.push(i.to_string());
            let next = incremental(&path, prefix, true).await?;
            assert_eq!(next, i + 1);
            path.pop();
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_incremental_file() -> Result<(), Box<dyn std::error::Error>>
    {
        let sqlmap_prefix = "q-";
        let ffuf_prefix = "z-";
        let addons = "addons";
        let mut path = PathBuf::from("/tmp/zxc_test/history/");
        for i in 1..11 {
            path.push(i.to_string());
            path.push(addons);
            let sql_next = incremental(&path, sqlmap_prefix, false).await?;
            assert_eq!(sql_next, i + 1);
            let ffuf_next = incremental(&path, ffuf_prefix, false).await?;
            assert_eq!(ffuf_next, i + 1);
            path.pop();
            path.pop();
        }
        Ok(())
    }
}
