use bytes::BytesMut;
use http_plz::OneRequest;

use super::Roneone;
use crate::proxy::handler_state::transition::update_frame::error::ProxyUpdateFrameError;
use crate::repeater::states::transition::bytes_to_frame::RepeaterBytesToFrame;

impl<T> RepeaterBytesToFrame for Roneone<T> {
    type Frame = OneRequest;

    fn parse_frame(
        &mut self,
        buf: BytesMut,
    ) -> Result<Self::Frame, ProxyUpdateFrameError> {
        Ok(OneRequest::try_from(buf)?)
    }

    fn frame_to_payload(&mut self, frame: Self::Frame) {
        self.payload = Some(frame.into_bytes())
    }
}
