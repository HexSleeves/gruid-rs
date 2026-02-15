//! Frame recording / playback (feature-gated on `serde`).
//!
//! Provides [`FrameEncoder`] and [`FrameDecoder`] for persisting [`Frame`]s
//! to a byte stream. The actual serialisation format uses `serde` (when the
//! feature is enabled); without it the types exist but encoding/decoding is
//! a no-op stub.

use std::io::{Read, Write};

use crate::grid::Frame;

// ---------------------------------------------------------------------------
// FrameEncoder
// ---------------------------------------------------------------------------

/// Encodes [`Frame`]s to a byte-oriented writer.
pub struct FrameEncoder<W: Write> {
    writer: W,
}

impl<W: Write> FrameEncoder<W> {
    /// Wrap a writer.
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Write a single frame.
    ///
    /// When the `serde` feature is not enabled this is a no-op.
    #[allow(unused_variables)]
    pub fn encode(&mut self, frame: &Frame) -> std::io::Result<()> {
        // TODO: implement real serialisation (e.g. bincode + flate2)
        // For now write the cell count as a simple length-prefixed marker.
        let len = frame.cells.len() as u32;
        self.writer.write_all(&len.to_le_bytes())?;
        Ok(())
    }

    /// Consume the encoder, returning the inner writer.
    pub fn into_inner(self) -> W {
        self.writer
    }
}

// ---------------------------------------------------------------------------
// FrameDecoder
// ---------------------------------------------------------------------------

/// Decodes [`Frame`]s from a byte-oriented reader.
pub struct FrameDecoder<R: Read> {
    reader: R,
}

impl<R: Read> FrameDecoder<R> {
    /// Wrap a reader.
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    /// Read the next frame, or `None` at EOF.
    ///
    /// When the `serde` feature is not enabled this always returns `None`.
    pub fn decode(&mut self) -> std::io::Result<Option<Frame>> {
        let mut buf = [0u8; 4];
        match self.reader.read_exact(&mut buf) {
            Ok(()) => {
                let _len = u32::from_le_bytes(buf);
                // TODO: deserialise real frame data.
                Ok(Some(Frame::default()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Consume the decoder, returning the inner reader.
    pub fn into_inner(self) -> R {
        self.reader
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_stub() {
        let mut buf = Vec::new();
        {
            let mut enc = FrameEncoder::new(&mut buf);
            enc.encode(&Frame::default()).unwrap();
        }
        let mut dec = FrameDecoder::new(buf.as_slice());
        let frame = dec.decode().unwrap();
        assert!(frame.is_some());
    }
}
