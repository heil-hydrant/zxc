use std::path::PathBuf;

use bytes::BytesMut;
use header_plz::OneResponseLine;
use http_plz::OneRequest;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::trace;

use super::Roneone;
use crate::io::*;
use crate::proxy::handler_state::handlers::read_http;
use crate::proxy::*;
use crate::repeater::prelude::*;

impl<T> AddRaw for Roneone<T> {
    fn add_raw(&mut self, buf: BytesMut) {
        self.payload = Some(buf);
    }
}

impl<T> ReadWrite for Roneone<T>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
{
    type Error = OneOneRWError;
    type State = RepeaterState<Self>;

    async fn read(mut self) -> Result<RepeaterState<Self>, OneOneRWError> {
        let frame =
            read_http::<T, OneResponseLine>(&mut self.stream, &mut self.buf)
                .await?;
        self.payload = Some(frame.into_bytes());
        trace!("read");
        Ok(RepeaterState::WriteResponse(self))
    }

    async fn write(mut self) -> Result<RepeaterState<Self>, OneOneRWError> {
        write_and_flush(&mut self.stream, self.payload.as_ref().unwrap())
            .await
            .map_err(OneOneRWError::Write)?;
        trace!("write");
        Ok(RepeaterState::Receive(self))
    }
}

impl<T> RepeaterBytesToFrame for Roneone<T> {
    type Frame = OneRequest;

    fn parse_frame(
        &mut self,
        buf: BytesMut,
    ) -> Result<Self::Frame, UpdateMsgError> {
        Ok(OneRequest::try_from(buf)?)
    }

    fn frame_to_payload(&mut self, frame: Self::Frame) {
        self.payload = Some(frame.into_bytes())
    }
}

impl<T> Rewrite for Roneone<T> {
    // If frame was uploaded, then rewrite
    #[inline]
    fn should_rewrite(&self) -> bool {
        self.update
    }
    fn get_write_data_and_file(&mut self) -> (&[u8], &mut File) {
        // safe to unwrap,
        // since data is set in read_from_file / update frame state
        (self.payload.as_ref().unwrap(), &mut self.file)
    }
}

const NEWRITE_ERROR: &str = "No new Write for http";
impl<T> Newrite for Roneone<T> {
    fn data_as_ref(&self) -> &[u8] {
        panic!("{}", NEWRITE_ERROR)
    }
    fn path_as_ref(&self) -> &PathBuf {
        panic!("{}", NEWRITE_ERROR)
    }

    fn update_path(&mut self) {
        panic!("{}", NEWRITE_ERROR)
    }
}

// Based on b:update variable set by user in UI
impl<T> ShouldUpdate for Roneone<T> {
    #[inline(always)]
    fn should_update(&self) -> bool {
        self.update
    }
}

impl<T> WriteResponse for Roneone<T> {
    // rep.res
    fn update_response_path(&mut self) {
        self.path
            .set_file_name(REPEATER_HTTP_FILENAME_RES);
    }

    fn response_path(&self) -> &PathBuf {
        &self.path
    }

    // safe to unwrap
    // since, payload set in read state
    fn response_data(&self) -> &[u8] {
        self.payload.as_ref().unwrap()
    }
}
