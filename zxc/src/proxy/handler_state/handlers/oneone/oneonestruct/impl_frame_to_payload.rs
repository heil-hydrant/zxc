use header_plz::body_headers::parse::ParseBodyHeaders;
use http_plz::OneMessageHead;

use super::*;
use crate::proxy::handler_state::FrameToPayload;

impl<T, E, U> FrameToPayload for OneOneStruct<T, E, U>
where
    U: OneInfoLine + std::fmt::Debug,
    OneMessageHead<U>: ParseBodyHeaders,
{
    fn frame_to_payload(&mut self) {
        // safe to unwrap
        self.payload = Some(self.frame.take().unwrap().into_bytes());
    }
}
