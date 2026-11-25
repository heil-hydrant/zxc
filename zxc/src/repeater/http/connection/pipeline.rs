use std::path::{Path, PathBuf};

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

// Based on b:update variable set by user in UI
impl<T> ShouldUpdate for Roneone<T> {
    #[inline(always)]
    fn should_update(&self) -> bool {
        self.update
    }
}

impl<T> AddRaw for Roneone<T> {
    fn add_raw(&mut self, buf: BytesMut) -> Result<(), UpdateMsgError> {
        self.payload = Some(buf);
        Ok(())
    }
}

impl<T> RepeaterBytesToFrame for Roneone<T> {
    type Frame = OneRequest;

    fn parse_frame(
        &mut self,
        buf: BytesMut,
    ) -> Result<Self::Frame, UpdateMsgError> {
        let mut req = OneRequest::try_from(buf)?;
        http_plz::NormalizerOpts::builder()
            .build()
            .normalize(&mut req);
        Ok(req)
    }

    fn frame_to_payload(&mut self, frame: Self::Frame) {
        self.payload = Some(frame.into_bytes())
    }
}

impl<T> Persist for Roneone<T> {
    // If frame was uploaded, then rewrite
    #[inline]
    fn should_rewrite(&self) -> bool {
        self.update
    }

    fn file_data_and_path(&mut self) -> (&mut File, &[u8], &Path) {
        // safe to unwrap,
        // since data is set in read_from_file / update frame state
        (&mut self.file, self.payload.as_ref().unwrap(), self.path.as_path())
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
