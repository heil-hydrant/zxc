use bytes::BytesMut;

use crate::proxy::*;

/* Description:
 *     Trait to convert Bytes to Frame (http/ws).
 *
 * Transition:
 *     UpdateFrame -> ReWrite
 */

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
