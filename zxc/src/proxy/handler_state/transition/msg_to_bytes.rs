// Trait to convert the frame (http req/res) to log data.
// Applicable to http only.

pub trait MsgToBytes {
    fn msg_to_bytes(&mut self);
}
