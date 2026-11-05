//! A Stream written through a piece a step keeps (`Kept`): what a unit
//! writes a name, a mark or a value at a time is gathered and written a
//! piece at a time, without a buffer of its own for every Stream.

use alloc::vec::Vec;
use std::io::{self, Write};

/// What is written gathered into `piece` and written to `inner` a piece at
/// a time: when it is full, and when it is flushed.
pub struct Writing<'a> {
    inner: &'a mut dyn Write,
    piece: &'a mut Vec<u8>,
}

impl<'a> Writing<'a> {
    /// `piece` emptied of what a Stream before this one left in it; it
    /// gathers up to its capacity.
    pub fn new(inner: &'a mut dyn Write, piece: &'a mut Vec<u8>) -> Self {
        piece.clear();
        Self { inner, piece }
    }

    fn write_piece(&mut self) -> io::Result<()> {
        self.inner.write_all(self.piece)?;
        self.piece.clear();
        Ok(())
    }
}

impl Write for Writing<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.piece.len() + bytes.len() > self.piece.capacity() {
            self.write_piece()?;
        }
        if bytes.len() >= self.piece.capacity() {
            return self.inner.write(bytes);
        }
        self.piece.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.write_piece()?;
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_written_arrives_a_piece_at_a_time_and_whole() {
        let mut piece = Vec::with_capacity(4);
        piece.extend_from_slice(b"left");
        let mut arrived = Vec::new();
        let mut writing = Writing::new(&mut arrived, &mut piece);
        for bytes in [&b"ab"[..], b"c", b"defghij", b"k"] {
            writing.write_all(bytes).expect("written");
        }
        writing.flush().expect("flushed");
        assert_eq!(arrived, b"abcdefghijk", "nothing a Stream before left");
    }
}
