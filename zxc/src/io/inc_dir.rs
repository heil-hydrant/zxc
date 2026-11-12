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
        let file_type = entry.file_type().await?;

        let dir_str = if for_dir && file_type.is_dir() {
            name
        } else if !for_dir && file_type.is_file() {
            let path: &Path = name.as_ref();
            let Some(stem) = path.file_stem() else {
                continue;
            };
            stem.into()
        } else {
            continue;
        };
        let high: usize = dir_str
            .to_str()
            .and_then(|s| s.strip_prefix(prefix))
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
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[tokio::test]
    async fn test_incremental_strict_prefix_and_entry_kind()
    -> Result<(), Box<dyn std::error::Error>> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "zxc_inc_dir_{}_{}",
            std::process::id(),
            nanos
        ));
        tokio::fs::create_dir_all(&dir).await?;

        tokio::fs::create_dir(dir.join("r-2")).await?;
        tokio::fs::create_dir(dir.join("x-r-9")).await?;
        tokio::fs::write(dir.join("r-5.req"), b"").await?;
        assert_eq!(incremental(&dir, "r-", true).await?, 3);

        tokio::fs::write(dir.join("q-3.req"), b"").await?;
        tokio::fs::write(dir.join("x-q-8.req"), b"").await?;
        tokio::fs::create_dir(dir.join("q-9")).await?;
        assert_eq!(incremental(&dir, "q-", false).await?, 4);

        tokio::fs::remove_dir_all(&dir).await?;
        Ok(())
    }
}
