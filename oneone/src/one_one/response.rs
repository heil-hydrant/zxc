use std::borrow::Cow;

use super::Body;
use crate::{OneOne, Response};

// OneOne response methods
impl OneOne<Response> {
    pub fn status_code(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self.header_struct.infoline().status())
    }

    pub fn content_length(&self) -> usize {
        if let Some(Body::Raw(body)) = &self.body {
            return body.len();
        }
        0
    }
}
