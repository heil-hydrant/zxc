use bytes::BytesMut;

use super::Roneone;
use crate::proxy::handler_state::transition::read_modified_file::AddRaw;

impl<T> AddRaw for Roneone<T> {
    fn add_raw(&mut self, buf: BytesMut) {
        self.payload = Some(buf);
    }
}
