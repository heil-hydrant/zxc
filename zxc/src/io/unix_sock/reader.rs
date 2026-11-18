use std::io::{Error, ErrorKind};

use bytes::BytesMut;
use tokio::net::UnixStream;

use crate::io::*;

pub fn read_from_unix(
    result: Result<(), Error>,
    sock: &mut UnixStream,
    buf: &mut BytesMut,
) -> Result<usize, UnixSockError> {
    if let Err(e) = result {
        if matches!(e.kind(), ErrorKind::WouldBlock) {
            return Err(UnixSockError::Block);
        } else {
            return Err(UnixSockError::Readable(e));
        }
    }
    match sock.try_read_buf(buf) {
        Ok(0) => Err(UnixSockError::EoF),
        Ok(n) => Ok(n),
        Err(e) => {
            if matches!(e.kind(), ErrorKind::WouldBlock) {
                Err(UnixSockError::Block)
            } else {
                Err(UnixSockError::Read(e))
            }
        }
    }
}

/// Splits the next newline-delimited JSON frame from `buf`.
///
/// Vim terminates every JSON channel message with a newline. A read can
/// contain several frames or only part of one, so the caller must keep the
/// returned tail until the next read.
///
/// # Returns
///
/// The unconsumed tail; `buf` keeps the complete frame. `None` if no
/// newline-terminated frame is buffered yet.
pub fn split_json_frame(buf: &mut BytesMut) -> Option<BytesMut> {
    let newline = buf.iter().position(|b| *b == b'\n')?;
    Some(buf.split_off(newline + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mirrors the runner loop: process every complete frame, keep the tail.
    fn drain_frames(buf: &mut BytesMut) -> Vec<BytesMut> {
        let mut frames = Vec::new();
        while let Some(tail) = split_json_frame(buf) {
            frames.push(buf.clone());
            *buf = tail;
        }
        frames
    }

    #[test]
    fn vim_channel_read_two_frames_in_one_read() {
        let mut buf = BytesMut::from(&b"[1,\"Toggle\"]\n[2,\"Close\"]\n"[..]);

        let frames = drain_frames(&mut buf);

        assert_eq!(frames.len(), 2);
        assert_eq!(&frames[0][..], b"[1,\"Toggle\"]\n");
        assert_eq!(&frames[1][..], b"[2,\"Close\"]\n");
        assert!(buf.is_empty());
    }

    #[test]
    fn vim_channel_read_frame_split_across_reads() {
        let mut buf = BytesMut::from(&b"[1,\"To"[..]);

        assert!(drain_frames(&mut buf).is_empty());
        assert_eq!(&buf[..], b"[1,\"To");

        buf.extend_from_slice(b"ggle\"]\n");
        let frames = drain_frames(&mut buf);

        assert_eq!(frames.len(), 1);
        assert_eq!(&frames[0][..], b"[1,\"Toggle\"]\n");
        assert!(buf.is_empty());
    }

    #[test]
    fn vim_channel_read_multiple_frames_and_partial_tail() {
        let mut buf =
            BytesMut::from(&b"[1,\"Toggle\"]\n[2,\"Close\"]\n[3,\"To"[..]);

        let frames = drain_frames(&mut buf);

        assert_eq!(frames.len(), 2);
        assert_eq!(&frames[0][..], b"[1,\"Toggle\"]\n");
        assert_eq!(&frames[1][..], b"[2,\"Close\"]\n");
        assert_eq!(&buf[..], b"[3,\"To");
    }

    #[test]
    fn vim_channel_read_escaped_newline_in_string() {
        let mut buf =
            BytesMut::from(&b"[1,{\"Encode\":{\"data\":\"a\\nb\"}}]\n"[..]);

        let frames = drain_frames(&mut buf);

        assert_eq!(frames.len(), 1);
        assert_eq!(&frames[0][..], b"[1,{\"Encode\":{\"data\":\"a\\nb\"}}]\n");
        let value: serde_json::Value =
            serde_json::from_slice(&frames[0]).unwrap();
        assert_eq!(value[1]["Encode"]["data"], "a\nb");
        assert!(buf.is_empty());
    }

    #[test]
    fn vim_channel_read_without_newline_retains_buffer() {
        let mut buf = BytesMut::from(&b"[1,\"Toggle\"]"[..]);

        assert!(drain_frames(&mut buf).is_empty());
        assert_eq!(&buf[..], b"[1,\"Toggle\"]");
    }

    #[test]
    fn vim_channel_read_empty_buffer() {
        let mut buf = BytesMut::new();

        assert!(drain_frames(&mut buf).is_empty());
        assert!(buf.is_empty());
    }
}
