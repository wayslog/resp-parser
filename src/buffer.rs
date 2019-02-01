use std::{cmp::min, io};

const BUFFER_SIZE: usize = 1024 * 1024 * 20;

/// Ring buffer for stream parse
#[derive(Debug)]
pub struct Buffer {
    head: usize,
    size: usize,
    scan: usize,
    data: Vec<u8>,
}

impl Default for Buffer {
    fn default() -> Self {
        Buffer {
            data: vec![0u8; BUFFER_SIZE],
            head: 0,
            size: 0,
            scan: 0,
        }
    }
}

pub(crate) struct Iter<'a> {
    buffer: &'a mut Buffer,
}

impl<'a> Iterator for Iter<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buffer.scan < self.buffer.size {
            let index = self.buffer.calc_index(self.buffer.scan);
            let result = self.buffer.data[index];
            self.buffer.scan += 1;
            Some(result)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let result = self.buffer.size - self.buffer.scan;
        (result, Some(result))
    }
}

impl Buffer {
    pub(crate) fn pop_scanned_buffer(&mut self) -> Vec<u8> {
        let mut result = Vec::with_capacity(self.scan);
        let data = ring_slice(&self.data, self.head, self.scan);
        result.extend_from_slice(data.0);
        result.extend_from_slice(data.1);

        self.head = self.calc_index(self.scan);
        self.size -= self.scan;
        self.scan = 0;

        result
    }

    pub(crate) fn iter(&mut self) -> Iter {
        Iter { buffer: self }
    }

    pub(crate) fn consume(&mut self, n: usize) -> usize {
        let old_scan = self.scan;
        self.scan += n;
        self.scan = min(self.scan, self.size);
        self.scan - old_scan
    }

    #[inline]
    fn calc_index(&self, length: usize) -> usize {
        assert!(length < self.data.len());
        let end = self.head + length;
        if end < self.data.len() {
            end
        } else {
            end - self.data.len()
        }
    }
}

impl io::Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.len() + self.size > self.data.len() {
            Err(io::ErrorKind::WriteZero)?
        }

        let begin = self.calc_index(self.size);
        let (first_dest, second_dest) = mut_ring_slice(self.data.as_mut(), begin, buf.len());
        let (first_src, second_src) = buf.split_at(first_dest.len());

        first_dest.copy_from_slice(first_src);
        second_dest.copy_from_slice(second_src);

        self.size += buf.len();

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn ring_slice<T>(data: &[T], begin: usize, length: usize) -> (&[T], &[T]) {
    assert!(length <= data.len());
    assert!(begin < data.len());

    let end = begin + length;
    if end < data.len() {
        (&data[begin..end], &[][..])
    } else {
        let end = end - data.len();
        (&data[begin..], &data[..end])
    }
}

fn mut_ring_slice<T>(data: &mut [T], begin: usize, length: usize) -> (&mut [T], &mut [T]) {
    assert!(length <= data.len());
    assert!(begin < data.len());

    let end = begin + length;
    if end < data.len() {
        (&mut data[begin..end], &mut [][..])
    } else {
        let end = end - data.len();
        let (left, first) = data.split_at_mut(begin);
        let (second, _) = left.split_at_mut(end);
        (first, second)
    }
}
