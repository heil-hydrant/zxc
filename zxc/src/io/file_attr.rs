use std::io::Error;
use std::path::PathBuf;

use tracing::trace;
use xattr::set;

use crate::core::constants::{
    XATTR_AUTHORITY, XATTR_H2_SCHEME, XATTR_HOST, XATTR_HTTP, XATTR_IS_H2,
    XATTR_SNI,
};
use crate::interceptor::H2Info;
use crate::proxy::ServerInfoJson;

pub async fn set_attr(
    path: &PathBuf,
    info: ServerInfoJson,
) -> Result<(), Error> {
    // spawn_blocking needs 'static data, so the borrowed path is cloned.
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || set_attr_sync(&path, info))
        .await
        .map_err(Error::other)?
}

fn set_attr_sync(path: &PathBuf, info: ServerInfoJson) -> Result<(), Error> {
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

pub async fn set_h2_attr(
    path: &PathBuf,
    server_info: ServerInfoJson,
    h2_info: Option<&H2Info>,
) -> Result<(), Error> {
    let path = path.to_owned();
    let h2_info = h2_info.cloned();
    tokio::task::spawn_blocking(move || {
        set_attr_sync(&path, server_info)?;
        set_h2_attr_sync(&path, h2_info.as_ref())
    })
    .await
    .map_err(Error::other)?
}

fn set_h2_attr_sync(
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

    fn server_info() -> ServerInfoJson {
        ServerInfoJson {
            host: "example.com:443".to_string(),
            http: None,
            sni: Some("example.com".to_string()),
        }
    }

    #[tokio::test]
    async fn test_set_attr() {
        let path = std::env::temp_dir()
            .join(format!("zxc_set_attr_{}.req", std::process::id()));
        let _ = std::fs::remove_file(&path);
        std::fs::File::create(&path).unwrap();

        let info = server_info();
        set_attr(&path, info).await.unwrap();

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

    #[tokio::test]
    async fn test_set_h2_attr() {
        let path = std::env::temp_dir()
            .join(format!("zxc_set_h2_attr_{}.req", std::process::id()));
        let _ = std::fs::remove_file(&path);
        std::fs::File::create(&path).unwrap();

        let info = H2Info::new(
            Some("example.com:443".to_string()),
            Some("https".to_string()),
        );
        set_h2_attr(&path, server_info(), Some(&info))
            .await
            .unwrap();

        assert_eq!(
            xattr::get(&path, XATTR_HOST).unwrap(),
            Some(b"example.com:443".to_vec())
        );
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

        std::fs::remove_file(&path).unwrap();
    }

    #[tokio::test]
    async fn test_set_h2_attr_without_pseudo_headers() {
        let path = std::env::temp_dir()
            .join(format!("zxc_set_h2_attr_empty_{}.req", std::process::id()));
        let _ = std::fs::remove_file(&path);
        std::fs::File::create(&path).unwrap();

        set_h2_attr(&path, server_info(), None)
            .await
            .unwrap();

        assert_eq!(
            xattr::get(&path, XATTR_HOST).unwrap(),
            Some(b"example.com:443".to_vec())
        );
        assert_eq!(
            xattr::get(&path, XATTR_IS_H2).unwrap(),
            Some(b"1".to_vec())
        );
        assert_eq!(xattr::get(&path, XATTR_AUTHORITY).unwrap(), None);
        assert_eq!(xattr::get(&path, XATTR_H2_SCHEME).unwrap(), None);

        std::fs::remove_file(&path).unwrap();
    }

    #[tokio::test]
    async fn test_set_h2_attr_preserves_empty_fields() {
        let path = std::env::temp_dir()
            .join(format!("zxc_set_h2_attr_empty_{}.req", std::process::id()));
        let _ = std::fs::remove_file(&path);
        std::fs::File::create(&path).unwrap();

        let info = H2Info::new(Some(String::new()), Some(String::new()));
        set_h2_attr(&path, server_info(), Some(&info))
            .await
            .unwrap();

        assert_eq!(
            xattr::get(&path, XATTR_HOST).unwrap(),
            Some(b"example.com:443".to_vec())
        );
        assert_eq!(
            xattr::get(&path, XATTR_IS_H2).unwrap(),
            Some(b"1".to_vec())
        );
        assert_eq!(
            xattr::get(&path, XATTR_AUTHORITY).unwrap(),
            Some(Vec::new())
        );
        assert_eq!(
            xattr::get(&path, XATTR_H2_SCHEME).unwrap(),
            Some(Vec::new())
        );

        std::fs::remove_file(&path).unwrap();
    }
}
