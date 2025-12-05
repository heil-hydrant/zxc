use std::borrow::Cow;

use crate::enums::request_methods::{CONNECT, Method};
use crate::{OneOne, Request};

// OneOne request methods
impl OneOne<Request> {
    pub fn is_connect_request(&self) -> bool {
        matches!(self.header_struct.infoline().method(), CONNECT)
    }

    pub fn method_as_string(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self.header_struct.infoline().method())
    }

    pub fn method_as_enum(&self) -> Method {
        self.header_struct
            .infoline()
            .method()
            .into()
    }

    pub fn uri_as_string(&self) -> Cow<'_, str> {
        self.header_struct
            .infoline()
            .uri_as_string()
    }
}
