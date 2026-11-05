//! A Stream read through a piece a step keeps (`Kept`), so a
//! reader that must leave what it did not use — a deflate stream ending
//! before a gzip trailer or a zip entry's descriptor — is buffered without
//! a buffer of its own for every Stream.

use std::io::{self, BufRead, Read};

/// `inner` read a piece at a time into `piece`.
pub struct Reading<'a> {
    inner: &'a mut dyn Read,
    piece: &'a mut [u8],
    at: usize,
    end: usize,
}

impl<'a> Reading<'a> {
    pub fn new(inner: &'a mut dyn Read, piece: &'a mut [u8]) -> Self {
        Self {
            inner,
            piece,
            at: 0,
            end: 0,
        }
    }
}

impl Read for Reading<'_> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        if self.at == self.end && into.len() >= self.piece.len() {
            return self.inner.read(into);
        }
        let held = self.fill_buf()?;
        let taken = held.len().min(into.len());
        into[..taken].copy_from_slice(&held[..taken]);
        self.consume(taken);
        Ok(taken)
    }
}

impl BufRead for Reading<'_> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.at == self.end {
            self.end = self.inner.read(self.piece)?;
            self.at = 0;
        }
        Ok(&self.piece[self.at..self.end])
    }

    fn consume(&mut self, used: usize) {
        self.at = (self.at + used).min(self.end);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[test]
    fn what_a_reader_leaves_is_read_next() {
        let mut source: &[u8] = b"abcdefgh";
        let mut piece = [0u8; 3];
        let mut buffered = Reading::new(&mut source, &mut piece);
        assert_eq!(buffered.fill_buf().expect("read"), b"abc");
        buffered.consume(2);
        let mut rest = Vec::new();
        buffered.read_to_end(&mut rest).expect("the rest");
        assert_eq!(rest, b"cdefgh");
    }
}
