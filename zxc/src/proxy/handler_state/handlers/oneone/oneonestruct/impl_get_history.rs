use oneone::{Request, Response};

use super::OneOneStruct;
use crate::proxy::handler_state::transition::write_history::{
    GetHistory, HistoryEnum, RequestHistory, ResponseHistory
};

impl<T, E> GetHistory for OneOneStruct<T, E, Request> {
    fn get_history(&self) -> HistoryEnum<'_> {
        let req = self.frame.as_ref().unwrap(); // safe to unwrap
        let method = req.method_as_string();
        let uri = req.uri_as_string();
        let host = self.server_info.address_to_string();
        let req =
            RequestHistory::new(self.log_id, method, self.scheme(), host, uri);
        HistoryEnum::Request(req)
    }
}

impl<T, E> GetHistory for OneOneStruct<T, E, Response> {
    fn get_history(&self) -> HistoryEnum<'_> {
        let res = self.frame.as_ref().unwrap(); // safe to unwrap
        let status_code = res.status_code();
        let content_length = res.content_length();
        let res =
            ResponseHistory::new(self.log_id, status_code, content_length);
        HistoryEnum::Response(res)
    }
}
