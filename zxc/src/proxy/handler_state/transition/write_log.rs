use std::io::Error;
use std::path::PathBuf;

use bytes::BytesMut;
use tokio::fs::File;
use tracing::trace;
use xattr::set;

use super::msg_to_bytes::MsgToBytes;
use crate::file_types::{EXT_REQ, EXT_RES};
use crate::interceptor::H2Info;
use crate::io::file::create_and_write_file;
use crate::proxy::prelude::*;

const XATTR_HOST: &str = "user.host";
const XATTR_HTTP: &str = "user.http";
const XATTR_SNI: &str = "user.sni";
const XATTR_IS_H2: &str = "user.is_h2";
const XATTR_AUTHORITY: &str = "user.authority";
const XATTR_H2_SCHEME: &str = "user.h2_scheme";

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
    set(path, XATTR_HOST, info.host.as_bytes())?;
    if info.http.is_some() {
        set(path, XATTR_HTTP, b"1")?;
    }
    if let Some(sni) = info.sni {
        set(path, XATTR_SNI, sni.as_bytes())?;
    }
    trace!("attr set");
    Ok(())
}

pub fn set_h2_attr(
    path: &PathBuf,
    info: Option<&H2Info>,
) -> Result<(), Error> {
    set(path, XATTR_IS_H2, b"1")?;
    if let Some(info) = info {
        if let Some(authority) = info.authority() {
            set(path, XATTR_AUTHORITY, authority.as_bytes())?;
        }
        if let Some(scheme) = info.scheme() {
            set(path, XATTR_H2_SCHEME, scheme.as_bytes())?;
        }
    }
    trace!("h2 attr set");
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

    #[test]
    fn test_set_h2_attr() {
        let path = std::env::temp_dir()
            .join(format!("zxc_set_h2_attr_{}.req", std::process::id()));
        let _ = std::fs::remove_file(&path);
        std::fs::File::create(&path).unwrap();

        let info = H2Info::new(
            Some("example.com:443".to_string()),
            Some("https".to_string()),
        );
        set_h2_attr(&path, Some(&info)).unwrap();

        assert_eq!(
            xattr::get(&path, XATTR_IS_H2).unwrap(),
            Some(b"1".to_vec())
        );
        assert_eq!(
            xattr::get(&path, XATTR_AUTHORITY).unwrap(),
            Some(b"example.com:443".to_vec())
        );
        assert_eq!(
            xattr::get(&path, XATTR_H2_SCHEME).unwrap(),
            Some(b"https".to_vec())
        );
        assert_eq!(xattr::get(&path, XATTR_HOST).unwrap(), None);

        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn test_set_h2_attr_without_pseudo_headers() {
        let path = std::env::temp_dir()
            .join(format!("zxc_set_h2_attr_empty_{}.req", std::process::id()));
        let _ = std::fs::remove_file(&path);
        std::fs::File::create(&path).unwrap();

        set_h2_attr(&path, None).unwrap();

        assert_eq!(
            xattr::get(&path, XATTR_IS_H2).unwrap(),
            Some(b"1".to_vec())
        );
        assert_eq!(xattr::get(&path, XATTR_AUTHORITY).unwrap(), None);
        assert_eq!(xattr::get(&path, XATTR_H2_SCHEME).unwrap(), None);

        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn test_set_h2_attr_ignores_empty_fields() {
        let path = std::env::temp_dir()
            .join(format!("zxc_set_h2_attr_skip_{}.req", std::process::id()));
        let _ = std::fs::remove_file(&path);
        std::fs::File::create(&path).unwrap();

        let info = H2Info::new(Some(String::new()), Some(String::new()));
        set_h2_attr(&path, Some(&info)).unwrap();

        assert_eq!(
            xattr::get(&path, XATTR_IS_H2).unwrap(),
            Some(b"1".to_vec())
        );
        assert_eq!(xattr::get(&path, XATTR_AUTHORITY).unwrap(), None);
        assert_eq!(xattr::get(&path, XATTR_H2_SCHEME).unwrap(), None);

        std::fs::remove_file(&path).unwrap();
    }
}
