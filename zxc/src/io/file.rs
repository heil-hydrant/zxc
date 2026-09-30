use std::io::SeekFrom;
use std::path::PathBuf;

use bytes::BytesMut;
use thiserror::Error;
use tokio::fs::{File, OpenOptions};
use tokio::io::{
    AsyncReadExt, AsyncSeekExt, {self},
};

use super::write::write_and_flush;

// File related io operations
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
