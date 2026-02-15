//! Frame recording / playback.
//!
//! Provides [`FrameEncoder`] and [`FrameDecoder`] for persisting [`Frame`]s
//! to a byte stream using a compact binary format. Matches Go gruid's
//! `recording.go` in functionality (gob+gzip) but uses a simple
//! length-prefixed binary encoding.
//!
//! ## Wire format
//!
//! Each frame is written as:
//! ```text
//! [total_byte_len: u32 LE]
//! [time_ms: u64 LE]
//! [width: i32 LE]
//! [height: i32 LE]
//! [num_cells: u32 LE]
//! for each cell:
//!   [pos.x: i32 LE] [pos.y: i32 LE]
//!   [ch: u32 LE]  (Unicode scalar value)
//!   [fg: u32 LE] [bg: u32 LE] [attrs: u32 LE]
//! ```

use std::io::{self, Read, Write};

use crate::cell::Cell;
use crate::geom::Point;
use crate::grid::{Frame, FrameCell};
use crate::style::{AttrMask, Color, Style};

/// Bytes per serialized cell: pos(8) + ch(4) + fg(4) + bg(4) + attrs(4) = 24
const CELL_SIZE: usize = 24;
/// Header size: time_ms(8) + width(4) + height(4) + num_cells(4) = 20
const HEADER_SIZE: usize = 20;

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
    pub fn encode(&mut self, frame: &Frame) -> io::Result<()> {
        let num_cells = frame.cells.len() as u32;
        let total_len = (HEADER_SIZE + frame.cells.len() * CELL_SIZE) as u32;

        // Length prefix
        self.writer.write_all(&total_len.to_le_bytes())?;

        // Header
        self.writer.write_all(&frame.time_ms.to_le_bytes())?;
        self.writer.write_all(&frame.width.to_le_bytes())?;
        self.writer.write_all(&frame.height.to_le_bytes())?;
        self.writer.write_all(&num_cells.to_le_bytes())?;

        // Cells
        for fc in &frame.cells {
            self.writer.write_all(&fc.pos.x.to_le_bytes())?;
            self.writer.write_all(&fc.pos.y.to_le_bytes())?;
            self.writer.write_all(&(fc.cell.ch as u32).to_le_bytes())?;
            self.writer.write_all(&fc.cell.style.fg.0.to_le_bytes())?;
            self.writer.write_all(&fc.cell.style.bg.0.to_le_bytes())?;
            self.writer
                .write_all(&fc.cell.style.attrs.0.to_le_bytes())?;
        }

        Ok(())
    }

    /// Flush the underlying writer.
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
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
    pub fn decode(&mut self) -> io::Result<Option<Frame>> {
        // Read length prefix
        let mut len_buf = [0u8; 4];
        match self.reader.read_exact(&mut len_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
        }
        let total_len = u32::from_le_bytes(len_buf) as usize;

        if total_len < HEADER_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "frame too small",
            ));
        }

        // Read the entire frame payload
        let mut data = vec![0u8; total_len];
        self.reader.read_exact(&mut data)?;

        // Parse header
        let time_ms = u64::from_le_bytes(data[0..8].try_into().unwrap());
        let width = i32::from_le_bytes(data[8..12].try_into().unwrap());
        let height = i32::from_le_bytes(data[12..16].try_into().unwrap());
        let num_cells = u32::from_le_bytes(data[16..20].try_into().unwrap()) as usize;

        let expected = HEADER_SIZE + num_cells * CELL_SIZE;
        if total_len != expected {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "frame size mismatch: expected {} bytes, got {}",
                    expected, total_len
                ),
            ));
        }

        // Parse cells
        let mut cells = Vec::with_capacity(num_cells);
        let mut offset = HEADER_SIZE;
        for _ in 0..num_cells {
            let x = i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
            let y = i32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap());
            let ch_u32 = u32::from_le_bytes(data[offset + 8..offset + 12].try_into().unwrap());
            let fg = u32::from_le_bytes(data[offset + 12..offset + 16].try_into().unwrap());
            let bg = u32::from_le_bytes(data[offset + 16..offset + 20].try_into().unwrap());
            let attrs = u32::from_le_bytes(data[offset + 20..offset + 24].try_into().unwrap());

            let ch = char::from_u32(ch_u32).unwrap_or('\u{FFFD}');

            cells.push(FrameCell {
                pos: Point::new(x, y),
                cell: Cell {
                    ch,
                    style: Style {
                        fg: Color(fg),
                        bg: Color(bg),
                        attrs: AttrMask(attrs),
                    },
                },
            });

            offset += CELL_SIZE;
        }

        Ok(Some(Frame {
            cells,
            width,
            height,
            time_ms,
        }))
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
    fn round_trip_empty_frame() {
        let frame = Frame {
            cells: vec![],
            width: 80,
            height: 24,
            time_ms: 1234,
        };

        let mut buf = Vec::new();
        {
            let mut enc = FrameEncoder::new(&mut buf);
            enc.encode(&frame).unwrap();
        }

        let mut dec = FrameDecoder::new(buf.as_slice());
        let decoded = dec.decode().unwrap().unwrap();
        assert_eq!(decoded.width, 80);
        assert_eq!(decoded.height, 24);
        assert_eq!(decoded.time_ms, 1234);
        assert!(decoded.cells.is_empty());

        // Next decode should be EOF
        assert!(dec.decode().unwrap().is_none());
    }

    #[test]
    fn round_trip_with_cells() {
        let frame = Frame {
            cells: vec![
                FrameCell {
                    pos: Point::new(5, 10),
                    cell: Cell {
                        ch: '@',
                        style: Style {
                            fg: Color::from_rgb(255, 0, 0),
                            bg: Color::from_rgb(0, 0, 255),
                            attrs: AttrMask::BOLD | AttrMask::UNDERLINE,
                        },
                    },
                },
                FrameCell {
                    pos: Point::new(0, 0),
                    cell: Cell::default(),
                },
            ],
            width: 40,
            height: 20,
            time_ms: 5000,
        };

        let mut buf = Vec::new();
        FrameEncoder::new(&mut buf).encode(&frame).unwrap();

        let decoded = FrameDecoder::new(buf.as_slice())
            .decode()
            .unwrap()
            .unwrap();

        assert_eq!(decoded.width, 40);
        assert_eq!(decoded.height, 20);
        assert_eq!(decoded.time_ms, 5000);
        assert_eq!(decoded.cells.len(), 2);

        // First cell
        assert_eq!(decoded.cells[0].pos, Point::new(5, 10));
        assert_eq!(decoded.cells[0].cell.ch, '@');
        assert_eq!(decoded.cells[0].cell.style.fg, Color::from_rgb(255, 0, 0));
        assert_eq!(decoded.cells[0].cell.style.bg, Color::from_rgb(0, 0, 255));
        assert!(decoded.cells[0]
            .cell
            .style
            .attrs
            .contains(AttrMask::BOLD));
        assert!(decoded.cells[0]
            .cell
            .style
            .attrs
            .contains(AttrMask::UNDERLINE));

        // Second cell — default
        assert_eq!(decoded.cells[1].pos, Point::new(0, 0));
        assert_eq!(decoded.cells[1].cell, Cell::default());
    }

    #[test]
    fn round_trip_multiple_frames() {
        let frames: Vec<Frame> = (0..5)
            .map(|i| Frame {
                cells: vec![FrameCell {
                    pos: Point::new(i, 0),
                    cell: Cell::default().with_char(char::from(b'A' + i as u8)),
                }],
                width: 80,
                height: 24,
                time_ms: i as u64 * 100,
            })
            .collect();

        let mut buf = Vec::new();
        {
            let mut enc = FrameEncoder::new(&mut buf);
            for f in &frames {
                enc.encode(f).unwrap();
            }
        }

        let mut dec = FrameDecoder::new(buf.as_slice());
        for (i, expected) in frames.iter().enumerate() {
            let decoded = dec.decode().unwrap().unwrap();
            assert_eq!(decoded.time_ms, expected.time_ms, "frame {i}");
            assert_eq!(decoded.cells.len(), 1, "frame {i}");
            assert_eq!(decoded.cells[0].cell.ch, expected.cells[0].cell.ch, "frame {i}");
        }
        assert!(dec.decode().unwrap().is_none());
    }

    #[test]
    fn unicode_round_trip() {
        let frame = Frame {
            cells: vec![FrameCell {
                pos: Point::new(0, 0),
                cell: Cell::default().with_char('\u{1F600}'), // 😀
            }],
            width: 1,
            height: 1,
            time_ms: 0,
        };

        let mut buf = Vec::new();
        FrameEncoder::new(&mut buf).encode(&frame).unwrap();
        let decoded = FrameDecoder::new(buf.as_slice())
            .decode()
            .unwrap()
            .unwrap();
        assert_eq!(decoded.cells[0].cell.ch, '\u{1F600}');
    }
}
