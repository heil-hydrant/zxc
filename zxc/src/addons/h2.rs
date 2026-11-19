//! Host header injection for HTTP/2 addon request copies.

use std::path::Path;

use bytes::BytesMut;
use http_plz::OneRequest;
use tokio::fs;

use crate::forward_info::{ForwardInfo, H2Info};
use crate::io::{FileErrorInfo, FileEvent};
use crate::proxy::server_info::json::ServerInfoJson;

use super::AddonError;

/// Ensures a `Host` header on a copied HTTP/2 request file.
///
/// # Steps
///
/// 1. Read the copied request file.
/// 2. Parse it into a [`OneRequest`].
/// 3. Pick the authority for the `Host` header.
/// 4. Insert `Host` when it is missing.
/// 5. Write the updated bytes back to the same file.
///
/// # Errors
///
/// - `AddonError::H2Request` [2] when the file is not a parsable request.
/// - `AddonError::H2Request` [3] when no authority is available.
/// - `AddonError::RequestFile` [1] [5] when the file cannot be read or
///   written.
pub async fn ensure_host_file(
    path: &Path,
    info: &ForwardInfo,
) -> Result<(), AddonError> {
    let name = path.to_path_buf();
    let bytes = fs::read(path).await.map_err(|error| {
        FileErrorInfo::from((&name, FileEvent::Read, error))
    })?;

    let mut request = OneRequest::try_from(BytesMut::from(&bytes[..]))
        .map_err(|e| AddonError::H2Request(e.to_string()))?;

    let host = request_host(info.h2_info.as_ref(), info.server_info.as_ref())
        .ok_or_else(|| AddonError::H2Request("missing host".to_string()))?;

    request.ensure_host(host);

    fs::write(path, request.into_bytes())
        .await
        .map_err(|error| {
            FileErrorInfo::from((&name, FileEvent::Write, error))
        })?;
    Ok(())
}

/// Returns the authority to use for the injected `Host` header.
///
/// The HTTP/2 `:authority` wins when present and non-empty because it keeps
/// non-default ports. Otherwise `server_info.host` is used.
#[inline(always)]
fn request_host<'a>(
    h2_info: Option<&'a H2Info>,
    server_info: Option<&'a ServerInfoJson>,
) -> Option<&'a str> {
    h2_info
        .and_then(H2Info::authority)
        .filter(|authority| !authority.is_empty())
        .or_else(|| server_info.map(|server_info| server_info.host.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server_info(host: &str) -> ServerInfoJson {
        ServerInfoJson {
            host: host.to_string(),
            http: None,
            sni: None,
        }
    }

    #[test]
    fn test_request_host_prefers_authority() {
        let h2 = H2Info::new(Some("internal.local:8443".to_string()), None);
        let server = server_info("public.example:443");

        assert_eq!(
            request_host(Some(&h2), Some(&server)),
            Some("internal.local:8443")
        );
    }

    #[test]
    fn test_request_host_falls_back_to_server_info() {
        let h2 = H2Info::new(Some(String::new()), None);
        let server = server_info("public.example:443");

        assert_eq!(
            request_host(Some(&h2), Some(&server)),
            Some("public.example:443")
        );
    }

    #[test]
    fn test_request_host_without_metadata() {
        assert_eq!(request_host(None, None), None);
    }
}
