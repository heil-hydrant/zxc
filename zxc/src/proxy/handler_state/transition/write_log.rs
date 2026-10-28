use std::io::Error;
use std::path::PathBuf;

use bytes::BytesMut;
use tokio::fs::File;
use tracing::trace;

use super::msg_to_bytes::MsgToBytes;
use crate::file_types::{EXT_REQ, EXT_RES};
use crate::io::file::create_and_write_file;
use crate::proxy::prelude::*;

const XATTR_HOST: &str = "user.host";
const XATTR_HTTP: &str = "user.http";
const XATTR_SNI: &str = "user.sni";

// Trait to perform file operations
pub trait FileOps {
    fn file_and_buf_as_mut(&mut self) -> (&mut File, &mut BytesMut);

    fn attach_file(&mut self, file: File);
}

// Trait to update the extension of the request/response.
pub trait UpdateLogExt {
    fn update_extension(&mut self);
}

#[inline]
pub fn update_http_req_ext(path: &mut Option<PathBuf>, log_id: usize) {
    let path = path.as_mut().unwrap();
    path.push(log_id.to_string());
    path.set_extension(EXT_REQ);
}

#[inline]
pub fn update_http_res_ext(path: &mut Option<PathBuf>) {
    path.as_mut()
        .unwrap()
        .set_extension(EXT_RES);
}

pub trait Log {
    fn path(&self) -> &PathBuf;

    fn log_data(&self) -> &[u8];
}

pub async fn write_log<T>(
    mut conn: T,
) -> Result<ProxyState<T>, ProxyStateError>
where
    T: UpdateLogExt + Log + FileOps + MsgToBytes,
{
    conn.update_extension();
    conn.msg_to_bytes();
    let file = create_and_write_file(conn.path(), conn.log_data()).await?;
    conn.attach_file(file);
    trace!("message logged| {}", conn.path().display());
    Ok(ProxyState::ShouldIntercept(conn))
}

pub fn set_attr(path: &PathBuf, info: ServerInfoJson) -> Result<(), Error> {
    xattr::set(path, XATTR_HOST, info.host.as_bytes())?;
    if info.http.is_some() {
        xattr::set(path, XATTR_HTTP, b"1")?;
    }
    if let Some(sni) = info.sni {
        xattr::set(path, XATTR_SNI, sni.as_bytes())?;
    }
    trace!("attr set");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_attr() {
        let path = std::env::temp_dir()
            .join(format!("zxc_set_attr_{}.req", std::process::id()));
        let _ = std::fs::remove_file(&path);
        std::fs::File::create(&path).unwrap();

        let info = ServerInfoJson {
            host: "example.com:443".to_string(),
            http: None,
            sni: Some("example.com".to_string()),
        };
        set_attr(&path, info).unwrap();

        assert_eq!(
            xattr::get(&path, XATTR_HOST).unwrap(),
            Some(b"example.com:443".to_vec())
        );
        assert_eq!(xattr::get(&path, XATTR_HTTP).unwrap(), None);
        assert_eq!(
            xattr::get(&path, XATTR_SNI).unwrap(),
            Some(b"example.com".to_vec())
        );

        std::fs::remove_file(&path).unwrap();
    }
}
