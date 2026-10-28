use std::path::PathBuf;

use bytes::BytesMut;
use header_plz::{RequestLine, ResponseLine};
use http_plz::{Message, OneRequest, OneResponse, Request, Response};
use tokio::fs::File;
use tokio::sync::mpsc::Sender;

use super::TwoConn;
use crate::commander::*;
use crate::history::*;
use crate::interceptor::*;
use crate::proxy::prelude::*;

// --- AddRaw ---
impl<T> AddRaw for TwoConn<T> {
    fn add_raw(&mut self, buf: BytesMut) {
        self.msg_bytes = Some(buf);
    }
}

// --- BytesToMsg ---
impl BytesToMsg for TwoConn<RequestLine> {
    type Msg = Message<RequestLine>;

    fn parse_bytes(&self, buf: BytesMut) -> Result<Self::Msg, UpdateMsgError> {
        let req = reparse_request(self.msg.as_ref(), buf)?;

        tracing::trace!(
            "h2 reparsed| authority={:?} scheme={:?}",
            req.authority(),
            req.scheme()
        );
        Ok(req)
    }

    fn add_msg(&mut self, msg: Self::Msg) {
        self.msg = Some(msg);
    }

    fn apply_h2_info(
        &mut self,
        h2_info: Option<&H2Info>,
    ) -> Result<(), UpdateMsgError> {
        let (Some(h2_info), Some(req)) = (h2_info, self.msg.as_mut()) else {
            return Ok(());
        };
        set_h2_pseudo_headers(req, h2_info)?;
        tracing::trace!(
            "h2 applied| authority={:?} scheme={:?}",
            req.authority(),
            req.scheme()
        );
        Ok(())
    }
}

impl BytesToMsg for TwoConn<ResponseLine> {
    type Msg = Message<ResponseLine>;

    fn parse_bytes(&self, buf: BytesMut) -> Result<Self::Msg, UpdateMsgError> {
        Ok(Response::try_from(OneResponse::try_from(buf)?)?)
    }

    fn add_msg(&mut self, msg: Self::Msg) {
        self.msg = Some(msg);
    }
}

// --- DropMsg ---
// close the stream if msg is dropped
impl<T> DropMsg for TwoConn<T> {
    #[inline(always)]
    fn continue_on_drop() -> Result<(), ProxyStateError> {
        Err(ProxyStateError::Drop)
    }
}

// --- MsgToBytes ---
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

// Apply :authority and :scheme to a request that was parsed from its
// origin-form log representation. Empty/absent values are left untouched.
fn set_h2_pseudo_headers(
    req: &mut Request,
    h2_info: &H2Info,
) -> Result<(), UpdateMsgError> {
    if let Some(authority) = h2_info
        .authority()
        .filter(|authority| !authority.is_empty())
    {
        req.try_set_authority(authority)?;
    }
    if let Some(scheme) = h2_info
        .scheme()
        .filter(|scheme| !scheme.is_empty())
    {
        req.try_set_scheme(scheme)?;
    }
    Ok(())
}

// The log file is origin-form, so reparsing it loses :authority and :scheme.
// Restore them from the request we already hold; any UI-provided h2_info is
// applied afterwards and takes precedence.
fn reparse_request(
    old: Option<&Request>,
    buf: BytesMut,
) -> Result<Request, UpdateMsgError> {
    let mut req = Request::try_from(OneRequest::try_from(buf)?)?;
    if let Some(old) = old {
        copy_h2_pseudo_headers(old, &mut req)?;
    }
    Ok(req)
}

// Copy pseudo headers from the request held before an origin-form log file
// is reparsed. This keeps them when the UI does not send h2_info.
fn copy_h2_pseudo_headers(
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

// --- Intercept ---
impl Intercept for TwoConn<RequestLine> {
    fn get_inter_info(&self) -> InterToUI {
        tracing::trace!("h2 intercepted| {:?}", self.msg.as_ref());
        let info = ServerInfoJson::from(&self.server_info);
        // Extract :authority and :scheme strictly from the request URI.
        // No fallback to HeaderMap or ServerInfo: absent pseudo-headers are
        // reported as None.
        let h2_info = self.msg.as_ref().and_then(|req| {
            let authority = req
                .authority()
                .filter(|authority| !authority.is_empty())
                .map(str::to_string);
            let scheme = req
                .scheme()
                .map(|scheme| scheme.as_str().to_string())
                .filter(|scheme| !scheme.is_empty());
            if authority.is_none() && scheme.is_none() {
                None
            } else {
                Some(H2Info::new(authority, scheme))
            }
        });
        InterToUI::build_h2_req(self.log_id, Some(info), h2_info)
    }
}

impl Intercept for TwoConn<ResponseLine> {
    fn get_inter_info(&self) -> InterToUI {
        InterToUI::build_http_res(self.log_id)
    }
}

// --- UpdateLogExt ---
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

// --- ReadWrite ---
impl ReadWrite for TwoConn<RequestLine> {
    type Error = ProxyStateError;
    type State = ProxyState<Self>;

    async fn read(self) -> Result<Self::State, Self::Error> {
        unreachable!()
    }

    async fn write(mut self) -> Result<Self::State, Self::Error> {
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
        self.send_response
            .send_response(self.msg.take().unwrap())
            .unwrap(); //TODO
        Ok(ProxyState::End(self))
    }
}

// --- Rewrite ---
impl<T> Rewrite for TwoConn<T> {
    fn file_and_data(&mut self) -> (&mut File, &[u8]) {
        (
            self.file
                .as_mut()
                .expect("file must be present"),
            self.msg_bytes
                .as_ref()
                .expect("payload must be present"),
        )
    }
}

// --- ShouldIntercept ---
impl ShouldIntercept for TwoConn<RequestLine> {}

impl ShouldIntercept for TwoConn<ResponseLine> {
    #[inline(always)]
    fn should_intercept(&self) -> Option<bool> {
        Some(self.need_response)
    }
}

// --- CanLog & ShouldLog & HttpRequestLog ---
impl<T> CanLog for TwoConn<T> {
    fn can_log(&self) -> bool {
        self.path.is_some()
    }
}

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
}

// --- ShouldRewrite ---
impl<T> ShouldRewrite for TwoConn<T> {
    #[inline(always)]
    fn should_rewrite(&self, resume_info: &ResumeInfo) -> bool {
        resume_info.update()
    }
}

// --- UpdateResumeInfo ---
impl UpdateResumeInfo for TwoConn<RequestLine> {
    fn update_resume_info(&mut self, resume_info: &ResumeInfo) {
        self.need_response = resume_info.need_response();
        // Apply pseudo-header edits in-place for the Send path, where the
        // frame is forwarded without being re-read from disk.
        if let Err(e) = self.apply_h2_info(resume_info.h2_info()) {
            tracing::error!("h2 info| {e}");
        }
    }
}

impl UpdateResumeInfo for TwoConn<ResponseLine> {
    #[inline(always)]
    fn update_resume_info(&mut self, _: &ResumeInfo) {}
}

// --- GetHistory ---
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
    fn test_set_h2_pseudo_headers_ignores_empty_fields() {
        let mut req = origin_form_request();
        let h2_info = H2Info::new(Some(String::new()), Some(String::new()));

        set_h2_pseudo_headers(&mut req, &h2_info).unwrap();

        assert!(req.authority().is_none());
        assert!(req.scheme().is_none());
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
        copy_h2_pseudo_headers(&old, &mut new).unwrap();

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
        copy_h2_pseudo_headers(&old, &mut new).unwrap();

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

        // Log files are origin-form, so this is what UpdateMsg parses.
        let buf = old.clone().into_one_rep().into_bytes();

        // Without the old request the pseudo headers are lost.
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
