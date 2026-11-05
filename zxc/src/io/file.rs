use std::io::SeekFrom;
use std::path::{Path, PathBuf};

use bytes::BytesMut;
use thiserror::Error;
use tokio::fs::{File, OpenOptions};
use tokio::io::{self, AsyncReadExt, AsyncSeekExt};

#[cfg(target_os = "linux")]
use crate::core::constants::REQUEST_XATTRS;
use crate::io::write_and_flush;

#[derive(Debug, Error)]
#[error("Event")]
pub enum FileEvent {
    Create,
    Write,
    Read,
    Open,
    Seek,
    SetLen,
}

#[derive(Debug, Error)]
#[error("file| {0}| {1}", self.name.to_string_lossy(), self.error)]
pub struct FileErrorInfo {
    name: PathBuf,
    event: FileEvent,
    error: io::Error,
}

impl From<(&PathBuf, FileEvent, io::Error)> for FileErrorInfo {
    fn from((name, event, error): (&PathBuf, FileEvent, io::Error)) -> Self {
        Self {
            name: name.to_path_buf(),
            event,
            error,
        }
    }
}

impl From<(&mut File, FileEvent, io::Error)> for FileErrorInfo {
    fn from((file, event, error): (&mut File, FileEvent, io::Error)) -> Self {
        let input = format!("{:?}", file);
        let path = if let Some(start) = input.find("path: \"") {
            // Find the start of ', read' after 'path: '
            if let Some(end) = input[start..].find("\", read") {
                // Extract the string between 'path: "' and '", read'
                &input[(start + 7)..(start + end + 1)]
            } else {
                &input
            }
        } else {
            &input
        };
        let name = PathBuf::from(path);
        Self {
            name,
            event,
            error,
        }
    }
}

pub async fn create_and_write_file(
    path: &PathBuf,
    contents: &[u8],
) -> Result<File, FileErrorInfo> {
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .truncate(true)
        .write(true)
        .open(path)
        .await
        .map_err(|e| FileErrorInfo::from((path, FileEvent::Open, e)))?;
    write_and_flush(&mut file, contents)
        .await
        .map_err(|e| FileErrorInfo::from((path, FileEvent::Write, e)))?;
    Ok(file)
}

/* Description:
 *      Copy a file and preserve the request metadata xattrs.
 *
 *      Linux: copy user.host, user.http, user.sni, user.is_h2,
 *      user.authority and user.h2_scheme.
 *
 *      macOS: std::fs::copy already preserves xattrs via
 *      fclonefileat/fcopyfile, so no explicit copy is needed.
 *
 *      Timestamps and ownership are not copied.
 */

pub async fn copy_with_request_metadata(
    from: impl AsRef<Path>,
    to: impl AsRef<Path>,
) -> io::Result<u64> {
    let from = from.as_ref().to_owned();
    let to = to.as_ref().to_owned();

    tokio::task::spawn_blocking(move || {
        let bytes = std::fs::copy(&from, &to)?;

        #[cfg(target_os = "linux")]
        {
            for name in REQUEST_XATTRS {
                if let Ok(Some(value)) = xattr::get(&from, name) {
                    let _ = xattr::set(&to, name, &value);
                }
            }
        }

        Ok(bytes)
    })
    .await
    .map_err(io::Error::other)?
}

pub async fn read_file(
    file: &mut File,
    buf: &mut BytesMut,
) -> Result<(), FileErrorInfo> {
    loop {
        match file.read_buf(buf).await {
            Ok(0) => break,
            Ok(_) => continue,
            Err(e) => {
                return Err(FileErrorInfo::from((file, FileEvent::Read, e)));
            }
        }
    }
    Ok(())
}

pub async fn rewrite_file(
    file: &mut File,
    buf: &[u8],
) -> Result<(), (FileEvent, io::Error)> {
    file.seek(SeekFrom::Start(0))
        .await
        .map_err(|e| (FileEvent::Seek, e))?;
    file.set_len(0)
        .await
        .map_err(|e| (FileEvent::SetLen, e))?;
    write_and_flush(file, buf)
        .await
        .map_err(|e| (FileEvent::Write, e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "zxc_copy_request_metadata_{}_{}",
            std::process::id(),
            nanos
        ))
    }

    #[tokio::test]
    async fn test_copy_with_request_metadata()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = unique_temp_dir();
        std::fs::create_dir_all(&dir)?;
        let src = dir.join("src.req");
        let dst = dir.join("dst.req");
        let contents = b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";
        std::fs::write(&src, contents)?;

        // macOS preserves xattrs in std::fs::copy. Linux copies the six
        // request xattrs explicitly in copy_with_request_metadata.
        let attrs: [(&str, &[u8]); 6] = [
            ("user.host", b"example.com:443".as_slice()),
            ("user.http", b"1".as_slice()),
            ("user.sni", b"example.com".as_slice()),
            ("user.is_h2", b"1".as_slice()),
            ("user.authority", b"".as_slice()),
            ("user.h2_scheme", b"https".as_slice()),
        ];
        let mut xattr_supported = true;
        for (name, value) in attrs {
            if xattr::set(&src, name, value).is_err() {
                xattr_supported = false;
                break;
            }
        }

        let bytes = copy_with_request_metadata(&src, &dst).await?;
        assert_eq!(bytes, contents.len() as u64);
        assert_eq!(std::fs::read(&dst)?, contents.to_vec());

        if xattr_supported {
            for (name, value) in attrs {
                assert_eq!(xattr::get(&dst, name)?, Some(value.to_vec()));
            }
        }

        std::fs::remove_dir_all(&dir)?;
        Ok(())
    }
}
