use header_plz::body_headers::parse::ParseBodyHeaders;
use http_plz::OneMessageHead;

use super::*;
use crate::proxy::handler_state::transition::BytesToFrame;
use crate::proxy::handler_state::transition::ProxyUpdateFrameError;

impl<T, E, U> BytesToFrame for OneOneStruct<T, E, U>
where
    U: OneInfoLine + std::fmt::Debug,
    OneMessageHead<U>: ParseBodyHeaders,
{
    type Frame = OneOne<U>;

    fn parse_frame(
        &self,
        buf: BytesMut,
    ) -> Result<Self::Frame, ProxyUpdateFrameError> {
        Ok(OneOne::<U>::try_from(buf)?)
    }

    fn add_frame(&mut self, frame: Self::Frame) {
        self.frame = Some(frame);
    }
}
