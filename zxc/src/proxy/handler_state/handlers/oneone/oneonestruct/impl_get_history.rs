use super::*;
use crate::proxy::handler_state::transition::write_history::{
    GetHistory, HistoryEnum, RequestHistory, ResponseHistory,
};

impl<T, E> GetHistory for OneOneStruct<T, E, OneRequestLine> {
    fn get_history(&self) -> HistoryEnum<'_> {
        let req = self.frame.as_ref().unwrap(); // safe to unwrap
        let method = req.method_enum().as_str().to_string();
        let uri = req.uri_as_string();
        let host = self.server_info.address_to_string();
        let req = RequestHistory::new(
            self.log_id,
            method.into(),
            self.scheme(),
            host,
            uri,
        );
        HistoryEnum::Request(req)
    }
}

impl<T, E> GetHistory for OneOneStruct<T, E, OneResponseLine> {
    fn get_history(&self) -> HistoryEnum<'_> {
        let res = self.frame.as_ref().unwrap(); // safe to unwrap
        let status_code = res
            .status()
            .map(|s| s.as_u16())
            .unwrap_or_default();
        let content_length = res
            .content_length()
            .and_then(|b| std::str::from_utf8(b).ok())
            .and_then(|s| s.trim().parse::<usize>().ok())
            .unwrap_or(0);
        let res =
            ResponseHistory::new(self.log_id, status_code, content_length);
        HistoryEnum::Response(res)
    }
}
