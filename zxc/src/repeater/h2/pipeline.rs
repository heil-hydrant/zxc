use std::path::PathBuf;

use bytes::BytesMut;
use http_plz::{NormalizerOpts, OneRequest, Request};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::trace;
use two_plz::client::ClientBuilder;

use super::Rh2;
use crate::CAPACITY_2MB;
use crate::forward_info::H2Info;
use crate::proxy::*;
use crate::repeater::prelude::*;

/* Description:
 *      Build an H2 request from the raw rep.req bytes.
 *
 * Steps:
 *      1. Parse the logged HTTP/1.1 style bytes into OneRequest
 *      2. If normalize is true, fix the body headers
 *      3. Convert OneRequest to the H2 Request
 *      4. Apply only the pseudo headers supplied by the UI
 *
 * Note:
 *      Normalization is only requested from parse_frame, which is reached
 *      when b:update is true. The write fallback reparses the payload without
 *      normalizing so b:update == false sends the user bytes unchanged.
 */

fn build_h2_request(
    buf: BytesMut,
    h2_info: Option<&H2Info>,
    normalize: bool,
) -> Result<Request, UpdateMsgError> {
    let mut one = OneRequest::try_from(buf)?;
    if normalize {
        NormalizerOpts::builder()
            .close_connection(false)
            .build()
            .normalize(&mut one);
    }
    let mut request = Request::try_from(one)?;
    if let Some(h2_info) = h2_info {
        h2_info.apply_to_request(&mut request)?;
    }
    Ok(request)
}

// Convert (RepeaterConn<T>, File) -> Rh2<T>
impl<T> From<(RepeaterConn<T>, File)> for Rh2<T> {
    fn from((conn, file): (RepeaterConn<T>, File)) -> Self {
        Self {
            buf: BytesMut::with_capacity(CAPACITY_2MB),
            path: conn.path,
            stream: Some(conn.stream),
            file,
            payload: None,
            update: conn.update,
            h2_info: conn.h2_info,
            pending_request: None,
            client_conn: None,
            response_future: None,
        }
    }
}

impl<T> RepeaterAddRaw for Rh2<T> {
    fn add_raw(&mut self, buf: BytesMut) -> Result<(), UpdateMsgError> {
        let request = build_h2_request(buf, self.h2_info.as_ref(), false)?;
        self.pending_request = Some(request);
        Ok(())
    }
}

impl<T> ReadWrite for Rh2<T>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
{
    type Error = ProxyStateError;
    type State = RepeaterState<Self>;

    async fn read(mut self) -> Result<RepeaterState<Self>, Self::Error> {
        let mut client_conn = self
            .client_conn
            .take()
            .ok_or(ProxyStateError::H2MissingRequest)?;
        let mut response_future = self
            .response_future
            .take()
            .ok_or(ProxyStateError::H2MissingRequest)?;

        // No tokio::spawn: keep driving the connection while waiting for the
        // single response so flow control and SETTINGS are still processed.
        let response = tokio::select! {
            response = &mut response_future => {
                response.map_err(|_| ProxyStateError::H2Closed)?
            }
            result = &mut client_conn => {
                result?;
                return Err(ProxyStateError::H2Closed);
            }
        };

        self.payload = Some(response.into_one_rep().into_bytes());
        trace!("read");
        Ok(RepeaterState::WriteResponse(self))
    }

    async fn write(mut self) -> Result<RepeaterState<Self>, Self::Error> {
        let request = self
            .pending_request
            .take()
            .ok_or(ProxyStateError::H2MissingRequest)?;
        let stream = self
            .stream
            .take()
            .ok_or(ProxyStateError::H2MissingRequest)?;
        let (client_conn, mut send_request) = ClientBuilder::new()
            .handshake(stream)
            .await?;
        let response_future = send_request.send_request(request)?;
        self.client_conn = Some(client_conn);
        self.response_future = Some(response_future);
        trace!("write");
        Ok(RepeaterState::Receive(self))
    }
}

impl<T> RepeaterBytesToFrame for Rh2<T> {
    type Frame = Request;

    fn parse_frame(
        &mut self,
        buf: BytesMut,
    ) -> Result<Self::Frame, UpdateMsgError> {
        build_h2_request(buf, self.h2_info.as_ref(), true)
    }

    fn frame_to_payload(&mut self, frame: Self::Frame) {
        let payload = frame
            .clone()
            .into_one_rep()
            .into_bytes();
        self.payload = Some(payload);
        self.pending_request = Some(frame);
    }
}

impl<T> Rewrite for Rh2<T> {
    // If frame was uploaded, then rewrite
    #[inline]
    fn should_rewrite(&self) -> bool {
        self.update
    }
    fn get_write_data_and_file(&mut self) -> (&[u8], &mut File) {
        // safe to unwrap,
        // since data is set in read_from_file / update frame state
        (self.payload.as_ref().unwrap(), &mut self.file)
    }
}

const NEWRITE_ERROR: &str = "No new Write for h2";
impl<T> Newrite for Rh2<T> {
    fn data_as_ref(&self) -> &[u8] {
        panic!("{}", NEWRITE_ERROR)
    }
    fn path_as_ref(&self) -> &PathBuf {
        panic!("{}", NEWRITE_ERROR)
    }

    fn update_path(&mut self) {
        panic!("{}", NEWRITE_ERROR)
    }
}

// Based on b:update variable set by user in UI
impl<T> ShouldUpdate for Rh2<T> {
    #[inline(always)]
    fn should_update(&self) -> bool {
        self.update
    }
}

impl<T> WriteResponse for Rh2<T> {
    // rep.res
    fn update_response_path(&mut self) {
        self.path
            .set_file_name(REPEATER_HTTP_FILENAME_RES);
    }

    fn response_path(&self) -> &PathBuf {
        &self.path
    }

    // safe to unwrap
    // since, payload set in read state
    fn response_data(&self) -> &[u8] {
        self.payload.as_ref().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use header_plz::const_headers::{CONNECTION, CONTENT_LENGTH};

    use super::*;

    const REQUEST: &str = "POST /submit HTTP/1.1\r\n\
                           Host: example.com\r\n\
                           Content-Length: 1\r\n\r\n\
                           hello";

    #[test]
    fn test_build_h2_request_normalizes_when_requested() {
        let h2_info = H2Info::new(
            Some("example.com:443".to_string()),
            Some("https".to_string()),
        );

        let req =
            build_h2_request(BytesMut::from(REQUEST), Some(&h2_info), true)
                .unwrap();

        assert_eq!(req.method().as_str(), "POST");
        assert_eq!(req.authority(), Some("example.com:443"));
        assert_eq!(req.scheme().map(|s| s.as_str()), Some("https"));
        assert_eq!(
            req.headers()
                .value_of_key(CONTENT_LENGTH),
            Some(&b"5"[..])
        );
        assert!(
            req.headers()
                .value_of_key(CONNECTION)
                .is_none()
        );
    }

    #[test]
    fn test_build_h2_request_without_normalize_keeps_raw_headers() {
        let req =
            build_h2_request(BytesMut::from(REQUEST), None, false).unwrap();

        assert_eq!(req.authority(), None);
        assert_eq!(req.scheme(), None);
        assert_eq!(
            req.headers()
                .value_of_key(CONTENT_LENGTH),
            Some(&b"1"[..])
        );
    }

    #[test]
    fn test_build_h2_request_applies_only_provided_pseudo_headers() {
        let raw = "GET https://original.test/path HTTP/1.1\r\n\
                   Host: original.test\r\n\r\n";
        let h2_info = H2Info::new(Some("changed.test:443".to_string()), None);

        let req = build_h2_request(BytesMut::from(raw), Some(&h2_info), false)
            .unwrap();

        assert_eq!(req.authority(), Some("changed.test:443"));
        assert_eq!(req.scheme().map(|s| s.as_str()), Some("https"));
    }

    #[test]
    fn test_build_h2_request_preserves_explicit_empty_scheme() {
        let h2_info = H2Info::new(None, Some(String::new()));

        let req =
            build_h2_request(BytesMut::from(REQUEST), Some(&h2_info), false)
                .unwrap();

        assert_eq!(req.scheme().map(|s| s.as_str()), Some(""));
    }
}
