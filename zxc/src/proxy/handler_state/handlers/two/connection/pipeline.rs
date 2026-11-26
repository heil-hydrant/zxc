use std::path::{Path, PathBuf};

use bytes::BytesMut;
use header_plz::{RequestLine, ResponseLine};
use http_plz::{OneRequest, OneResponse, Request, Response};
use tokio::fs::File;
use tokio::sync::mpsc::Sender;

use super::TwoConn;
use crate::commander::*;
use crate::history::*;
use crate::interceptor::*;
use crate::proxy::prelude::*;

// ReadWrite
impl ReadWrite for TwoConn<RequestLine> {
    type Error = ProxyStateError;
    type State = ProxyState<Self>;

    async fn read(self) -> Result<Self::State, Self::Error> {
        unreachable!()
    }

    async fn write(mut self) -> Result<Self::State, Self::Error> {
        self.h2_pseudo = self
            .msg
            .as_ref()
            .and_then(request_h2_info);
        let msg = self.msg.take().unwrap();
        tracing::trace!("h2 outbound| {:?}", msg);
        let response_future = self.send_request.send_request(msg)?;
        self.recv_response = Some(response_future);
        Ok(ProxyState::End(self))
    }
}

impl ReadWrite for TwoConn<ResponseLine> {
    type Error = ProxyStateError;
    type State = ProxyState<Self>;

    async fn read(mut self) -> Result<Self::State, Self::Error> {
        let response_future = self.recv_response.take().unwrap();
        let response = match response_future.await {
            Ok(resp) => resp,
            Err(partial) => {
                // TODO
                let err = partial.err().to_string();
                return Err(
                    OneOneRWError::Read(std::io::Error::other(err)).into()
                );
            }
        };
        self.msg = Some(response);
        Ok(ProxyState::ShouldLog(self))
    }

    async fn write(mut self) -> Result<Self::State, Self::Error> {
        let msg = self
            .msg
            .take()
            .ok_or(Http2Error::MissingRequest)?;
        self.send_response.send_response(msg)?;
        Ok(ProxyState::End(self))
    }
}

// CanLog
impl<T> CanLog for TwoConn<T> {
    fn can_log(&self) -> bool {
        self.path.is_some()
    }
}

// ShouldLog & HttpRequestLog
impl HttpRequestLog for TwoConn<RequestLine> {
    fn id(&self) -> SoldierId {
        self.id
    }

    fn log_id_mut(&mut self) -> &mut usize {
        &mut self.log_id
    }

    fn path_mut(&mut self) -> &mut Option<PathBuf> {
        &mut self.path
    }

    fn history_sendr_mut(
        &mut self,
    ) -> &mut Option<Sender<CommanderToHistory>> {
        &mut self.history_sendr
    }

    fn http_commander_log_request(&self) -> Option<CommanderRequest> {
        let msg = self.msg.as_ref().unwrap();
        http_log_request(self.id, msg.method(), msg.headers(), || msg.path())
    }
}

impl ShouldLog for TwoConn<ResponseLine> {
    type LogResult = ();

    #[inline(always)]
    fn commander_log_request(&self) -> Option<CommanderRequest> {
        None
    }
}

// MsgToBytes
impl MsgToBytes for TwoConn<RequestLine> {
    fn msg_to_bytes(&mut self) {
        self.msg_bytes = Some(
            self.msg
                .as_ref()
                .unwrap()
                .clone()
                .into_one_rep()
                .into_bytes(),
        );
    }
}

impl MsgToBytes for TwoConn<ResponseLine> {
    fn msg_to_bytes(&mut self) {
        self.msg_bytes = Some(
            self.msg
                .as_ref()
                .unwrap()
                .clone()
                .into_one_rep()
                .into_bytes(),
        );
    }
}

// GetHistory
impl GetHistory for TwoConn<RequestLine> {
    fn get_history(&self) -> HistoryEnum<'_> {
        let msg = self.msg.as_ref().unwrap();
        HistoryEnum::http_request(
            self.log_id,
            &self.server_info,
            msg.method().as_str(),
            msg.path_and_query().as_str(),
        )
    }
}

impl GetHistory for TwoConn<ResponseLine> {
    fn get_history(&self) -> HistoryEnum<'_> {
        let msg = self.msg.as_ref().unwrap();
        HistoryEnum::http_response(
            self.log_id,
            msg.status().as_u16(),
            msg.content_length(),
        )
    }
}

// UpdateLogExt
impl UpdateLogExt for TwoConn<RequestLine> {
    fn update_extension(&mut self) {
        update_http_req_ext(&mut self.path, self.log_id);
    }
}

impl UpdateLogExt for TwoConn<ResponseLine> {
    fn update_extension(&mut self) {
        update_http_res_ext(&mut self.path);
    }
}

// ShouldIntercept
impl ShouldIntercept for TwoConn<RequestLine> {}

impl ShouldIntercept for TwoConn<ResponseLine> {
    #[inline(always)]
    fn should_intercept(&self) -> Option<bool> {
        Some(self.need_response)
    }
}

// Intercept
impl Intercept for TwoConn<RequestLine> {
    fn get_inter_info(&self) -> InterToUI {
        let info = ServerInfoJson::from(&self.server_info);
        let h2_info = self
            .msg
            .as_ref()
            .and_then(request_h2_info);
        InterToUI::build_h2_req(self.log_id, Some(info), h2_info)
    }
}

impl Intercept for TwoConn<ResponseLine> {
    fn get_inter_info(&self) -> InterToUI {
        InterToUI::build_http_res(self.log_id)
    }
}

// DropMsg
// HTTP/2 closes the stream if the message is dropped.
impl<T> DropMsg for TwoConn<T> {}

// UpdateResumeInfo
impl UpdateResumeInfo for TwoConn<RequestLine> {
    fn update_resume_info(
        &mut self,
        resume_info: &ResumeInfo,
    ) -> Result<(), UpdateMsgError> {
        self.need_response = resume_info.need_response();
        if let (Some(h2_info), Some(req)) =
            (resume_info.h2_info(), self.msg.as_mut())
        {
            set_h2_pseudo_headers(req, h2_info)?;
            tracing::trace!(
                "h2 applied| authority={:?} scheme={:?}",
                req.authority(),
                req.scheme()
            );
        }
        Ok(())
    }
}

impl UpdateResumeInfo for TwoConn<ResponseLine> {
    #[inline(always)]
    fn update_resume_info(
        &mut self,
        _: &ResumeInfo,
    ) -> Result<(), UpdateMsgError> {
        Ok(())
    }
}

// AddRaw
impl AddRaw for TwoConn<RequestLine> {
    fn add_raw(&mut self, buf: BytesMut) -> Result<(), UpdateMsgError> {
        self.msg = Some(reparse_request(self.msg.as_ref(), buf)?);
        Ok(())
    }
}

impl AddRaw for TwoConn<ResponseLine> {
    fn add_raw(&mut self, buf: BytesMut) -> Result<(), UpdateMsgError> {
        self.msg = Some(Response::try_from(OneResponse::try_from(buf)?)?);
        Ok(())
    }
}

// BytesToMsg
impl BytesToMsg for TwoConn<RequestLine> {
    fn parse_bytes(&mut self, buf: BytesMut) -> Result<(), UpdateMsgError> {
        self.msg = Some(reparse_request(self.msg.as_ref(), buf)?);
        Ok(())
    }
}

impl BytesToMsg for TwoConn<ResponseLine> {
    fn parse_bytes(&mut self, buf: BytesMut) -> Result<(), UpdateMsgError> {
        self.msg = Some(Response::try_from(OneResponse::try_from(buf)?)?);
        Ok(())
    }
}

fn request_h2_info(req: &Request) -> Option<H2Info> {
    let authority = req
        .authority()
        .filter(|authority| !authority.is_empty())
        .map(str::to_string);
    let scheme = req
        .scheme()
        .map(|scheme| scheme.as_str().to_string());

    if authority.is_none() && scheme.is_none() {
        None
    } else {
        Some(H2Info::new(authority, scheme))
    }
}

// ShouldRewrite
impl<T> ShouldRewrite for TwoConn<T> {
    #[inline(always)]
    fn should_rewrite(&self, resume_info: &ResumeInfo) -> bool {
        resume_info.update()
    }
}

// Rewrite
impl<T> Rewrite for TwoConn<T> {
    fn file_data_and_path(&mut self) -> (&mut File, &[u8], &Path) {
        (
            self.file.as_mut().unwrap(),
            self.msg_bytes.as_ref().unwrap(),
            self.path.as_deref().unwrap(),
        )
    }
}

// Helper functions
fn set_h2_pseudo_headers(
    req: &mut Request,
    h2_info: &H2Info,
) -> Result<(), UpdateMsgError> {
    h2_info.apply_to_request(req)?;
    Ok(())
}

fn reparse_request(
    old: Option<&Request>,
    buf: BytesMut,
) -> Result<Request, UpdateMsgError> {
    let mut req = Request::try_from(OneRequest::try_from(buf)?)?;
    if let Some(old) = old {
        copy_request_pseudo_headers(old, &mut req)?;
    }
    Ok(req)
}

#[inline(always)]
fn copy_request_pseudo_headers(
    from: &Request,
    to: &mut Request,
) -> Result<(), UpdateMsgError> {
    if let Some(authority) = from.authority() {
        to.try_set_authority(authority)?;
    }
    if let Some(scheme) = from.scheme() {
        to.try_set_scheme(scheme.as_str())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin_form_request() -> Request {
        Request::try_from(
            OneRequest::try_from(BytesMut::from(
                "GET /path?key=value HTTP/2\r\n\r\n",
            ))
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn test_request_h2_info_extracts_pseudo_headers() {
        let mut req = origin_form_request();
        req.try_set_authority("internal.local:8443")
            .unwrap();
        req.try_set_scheme("https").unwrap();

        let info = request_h2_info(&req).unwrap();

        assert_eq!(info.authority(), Some("internal.local:8443"));
        assert_eq!(info.scheme(), Some("https"));
    }

    #[test]
    fn test_request_h2_info_partial() {
        let mut req = origin_form_request();
        req.try_set_authority("internal.local")
            .unwrap();

        let info = request_h2_info(&req).unwrap();

        assert_eq!(info.authority(), Some("internal.local"));
        assert!(info.scheme().is_none());
    }

    #[test]
    fn test_request_h2_info_preserves_empty_scheme() {
        let mut req = origin_form_request();
        req.try_set_scheme("").unwrap();

        let info = request_h2_info(&req).unwrap();

        assert!(info.authority().is_none());
        assert_eq!(info.scheme(), Some(""));
    }

    #[test]
    fn test_request_h2_info_without_pseudo_headers() {
        let req = origin_form_request();

        assert!(request_h2_info(&req).is_none());
    }

    #[test]
    fn test_set_h2_pseudo_headers() {
        let mut req = origin_form_request();
        assert!(req.authority().is_none());
        assert!(req.scheme().is_none());

        let h2_info = H2Info::new(
            Some("internal.local:8443".to_string()),
            Some("https".to_string()),
        );
        set_h2_pseudo_headers(&mut req, &h2_info).unwrap();

        assert_eq!(req.authority(), Some("internal.local:8443"));
        assert_eq!(
            req.scheme()
                .map(|scheme| scheme.as_str()),
            Some("https")
        );
        assert_eq!(req.path(), "/path");
        assert_eq!(req.query(), Some("key=value"));
    }

    #[test]
    fn test_set_h2_pseudo_headers_preserves_empty_values() {
        let mut req = origin_form_request();
        req.try_set_authority("old.local")
            .unwrap();
        req.try_set_scheme("https").unwrap();

        let h2_info = H2Info::new(Some(String::new()), Some(String::new()));

        set_h2_pseudo_headers(&mut req, &h2_info).unwrap();

        assert!(req.authority().is_none());
        assert_eq!(
            req.scheme()
                .map(|scheme| scheme.as_str()),
            Some("")
        );
    }

    #[test]
    fn test_set_h2_pseudo_headers_none_leaves_existing_values() {
        let mut req = origin_form_request();
        req.try_set_authority("old.local")
            .unwrap();
        req.try_set_scheme("https").unwrap();

        let h2_info = H2Info::new(None, None);

        set_h2_pseudo_headers(&mut req, &h2_info).unwrap();

        assert_eq!(req.authority(), Some("old.local"));
        assert_eq!(
            req.scheme()
                .map(|scheme| scheme.as_str()),
            Some("https")
        );
    }

    #[test]
    fn test_set_h2_pseudo_headers_partial() {
        let mut req = origin_form_request();
        let h2_info = H2Info::new(Some("internal.local".to_string()), None);

        set_h2_pseudo_headers(&mut req, &h2_info).unwrap();

        assert_eq!(req.authority(), Some("internal.local"));
        assert!(req.scheme().is_none());
    }

    #[test]
    fn test_copy_h2_pseudo_headers() {
        let mut old = origin_form_request();
        old.try_set_authority("internal.local:8443")
            .unwrap();
        old.try_set_scheme("https").unwrap();

        let mut new = origin_form_request();
        copy_request_pseudo_headers(&old, &mut new).unwrap();

        assert_eq!(new.authority(), Some("internal.local:8443"));
        assert_eq!(
            new.scheme()
                .map(|scheme| scheme.as_str()),
            Some("https")
        );
    }

    #[test]
    fn test_h2_info_overrides_copied_pseudo_headers() {
        let mut old = origin_form_request();
        old.try_set_authority("internal.local:8443")
            .unwrap();
        old.try_set_scheme("https").unwrap();

        let mut new = origin_form_request();
        copy_request_pseudo_headers(&old, &mut new).unwrap();

        let h2_info = H2Info::new(
            Some("override.local".to_string()),
            Some("http".to_string()),
        );
        set_h2_pseudo_headers(&mut new, &h2_info).unwrap();

        assert_eq!(new.authority(), Some("override.local"));
        assert_eq!(
            new.scheme()
                .map(|scheme| scheme.as_str()),
            Some("http")
        );
    }

    #[test]
    fn test_reparse_preserves_h2_pseudo_headers() {
        let mut old = origin_form_request();
        old.try_set_authority("www.google.com")
            .unwrap();
        old.try_set_scheme("https").unwrap();

        let buf = old.clone().into_one_rep().into_bytes();

        let without_old = reparse_request(None, buf.clone()).unwrap();
        assert!(without_old.authority().is_none());
        assert!(without_old.scheme().is_none());

        let reparsed = reparse_request(Some(&old), buf).unwrap();

        assert_eq!(reparsed.authority(), Some("www.google.com"));
        assert_eq!(
            reparsed
                .scheme()
                .map(|scheme| scheme.as_str()),
            Some("https")
        );
        assert_eq!(reparsed.path(), "/path");
        assert_eq!(reparsed.query(), Some("key=value"));
    }
}
