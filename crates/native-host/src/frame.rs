//! Native Messaging framing: a 32-bit length in native byte order followed by
//! that many bytes of UTF-8 JSON. The pipe between the host and the desktop
//! app reuses it, so one reader and one writer cover both legs.

use std::io::{self, Read, Write};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Largest message accepted in either direction. Real messages are a few
/// hundred bytes; Chrome would allow 64 MiB, which a broken or hostile peer
/// could use to make the host allocate that much per message.
pub const MAX_MESSAGE_LEN: usize = 64 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("a {0}-byte message is larger than the {MAX_MESSAGE_LEN}-byte limit")]
    TooLarge(u64),
    #[error(transparent)]
    Io(#[from] io::Error),
}

/// Read one frame. `Ok(None)` means the peer closed the stream cleanly
/// between frames; a stream that ends inside a frame is an error.
pub fn read_frame(input: &mut impl Read) -> Result<Option<Vec<u8>>, FrameError> {
    let mut prefix = [0u8; 4];
    let mut filled = 0;
    while filled < prefix.len() {
        match input.read(&mut prefix[filled..]) {
            Ok(0) if filled == 0 => return Ok(None),
            Ok(0) => return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into()),
            Ok(n) => filled += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
        }
    }
    let len = checked_len(prefix)?;
    let mut payload = vec![0; len];
    input.read_exact(&mut payload)?;
    Ok(Some(payload))
}

/// Write one frame and flush it, so the peer sees it immediately.
pub fn write_frame(output: &mut impl Write, payload: &[u8]) -> Result<(), FrameError> {
    let prefix = prefix_for(payload)?;
    output.write_all(&prefix)?;
    output.write_all(payload)?;
    output.flush()?;
    Ok(())
}

pub async fn read_frame_async(
    input: &mut (impl AsyncRead + Unpin),
) -> Result<Option<Vec<u8>>, FrameError> {
    let mut prefix = [0u8; 4];
    let mut filled = 0;
    while filled < prefix.len() {
        match input.read(&mut prefix[filled..]).await? {
            0 if filled == 0 => return Ok(None),
            0 => return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into()),
            n => filled += n,
        }
    }
    let len = checked_len(prefix)?;
    let mut payload = vec![0; len];
    input.read_exact(&mut payload).await?;
    Ok(Some(payload))
}

pub async fn write_frame_async(
    output: &mut (impl AsyncWrite + Unpin),
    payload: &[u8],
) -> Result<(), FrameError> {
    let prefix = prefix_for(payload)?;
    output.write_all(&prefix).await?;
    output.write_all(payload).await?;
    output.flush().await?;
    Ok(())
}

/// The length is checked before anything is allocated.
fn checked_len(prefix: [u8; 4]) -> Result<usize, FrameError> {
    let len = u32::from_ne_bytes(prefix);
    if len as usize > MAX_MESSAGE_LEN {
        return Err(FrameError::TooLarge(len.into()));
    }
    Ok(len as usize)
}

fn prefix_for(payload: &[u8]) -> Result<[u8; 4], FrameError> {
    if payload.len() > MAX_MESSAGE_LEN {
        return Err(FrameError::TooLarge(payload.len() as u64));
    }
    Ok((payload.len() as u32).to_ne_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn framed(payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        write_frame(&mut out, payload).unwrap();
        out
    }

    #[test]
    fn round_trips_several_frames() {
        let mut stream = framed(br#"{"a":1}"#);
        stream.extend(framed(b""));
        stream.extend(framed(br#"{"b":2}"#));
        let mut input = stream.as_slice();
        assert_eq!(read_frame(&mut input).unwrap().unwrap(), br#"{"a":1}"#);
        assert_eq!(read_frame(&mut input).unwrap().unwrap(), b"");
        assert_eq!(read_frame(&mut input).unwrap().unwrap(), br#"{"b":2}"#);
        assert!(read_frame(&mut input).unwrap().is_none());
    }

    #[test]
    fn prefix_is_native_endian_length() {
        let out = framed(b"hello");
        assert_eq!(&out[..4], &5u32.to_ne_bytes());
        assert_eq!(&out[4..], b"hello");
    }

    #[test]
    fn stream_ending_inside_a_frame_is_an_error() {
        let full = framed(b"hello");
        for cut in [2, 4, 7] {
            let mut input = &full[..cut];
            match read_frame(&mut input) {
                Err(FrameError::Io(e)) => assert_eq!(e.kind(), io::ErrorKind::UnexpectedEof),
                other => panic!("cut at {cut}: {other:?}"),
            }
        }
    }

    #[test]
    fn oversized_length_is_rejected_before_reading_the_body() {
        let len = (MAX_MESSAGE_LEN as u32 + 1).to_ne_bytes();
        // No body follows: the reader must not try to read (or allocate) it.
        let mut input = len.as_slice();
        assert!(matches!(
            read_frame(&mut input),
            Err(FrameError::TooLarge(n)) if n == MAX_MESSAGE_LEN as u64 + 1
        ));
        let huge = u32::MAX.to_ne_bytes();
        assert!(matches!(
            read_frame(&mut huge.as_slice()),
            Err(FrameError::TooLarge(_))
        ));
    }

    #[test]
    fn limit_itself_is_allowed_and_writes_are_capped() {
        let payload = vec![b'x'; MAX_MESSAGE_LEN];
        let out = framed(&payload);
        assert_eq!(
            read_frame(&mut out.as_slice()).unwrap().unwrap().len(),
            MAX_MESSAGE_LEN
        );
        let too_big = vec![b'x'; MAX_MESSAGE_LEN + 1];
        assert!(matches!(
            write_frame(&mut Vec::new(), &too_big),
            Err(FrameError::TooLarge(_))
        ));
    }

    #[tokio::test]
    async fn async_reader_and_writer_match_the_sync_ones() {
        let mut out = Vec::new();
        write_frame_async(&mut out, br#"{"x":true}"#).await.unwrap();
        assert_eq!(out, framed(br#"{"x":true}"#));
        let mut input = out.as_slice();
        assert_eq!(
            read_frame_async(&mut input).await.unwrap().unwrap(),
            br#"{"x":true}"#
        );
        assert!(read_frame_async(&mut input).await.unwrap().is_none());
        let len = (MAX_MESSAGE_LEN as u32 + 1).to_ne_bytes();
        assert!(matches!(
            read_frame_async(&mut len.as_slice()).await,
            Err(FrameError::TooLarge(_))
        ));
    }
}
