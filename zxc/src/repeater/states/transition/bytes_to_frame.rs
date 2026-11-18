use bytes::BytesMut;

use crate::proxy::*;

/// Converts buffered bytes into an HTTP or WebSocket frame.
///
/// # Transitions
///
/// `UpdateFrame` -> `ReWrite`.
pub trait RepeaterBytesToFrame {
    type Frame;

    // Convert buf to Frame
    fn parse_frame(
        &mut self,
        buf: BytesMut,
    ) -> Result<Self::Frame, UpdateMsgError>;

    // Convert Frame to payload
    fn frame_to_payload(&mut self, frame: Self::Frame);
}
