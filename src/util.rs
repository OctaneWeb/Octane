use std::io::Read;
use std::iter::FusedIterator;
use std::ops::Deref;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, Result};

pub fn find_in_slice<T: Eq>(haystack: &[T], needle: &[T]) -> Option<usize> {
    // naive algorithm only meant for small needles
    if needle.len() > haystack.len() {
        return None;
    }
    for i in 0..=haystack.len() - needle.len() {
        let mut matching = true;
        for j in 0..needle.len() {
            if haystack[i + j] != needle[j] {
                matching = false;
                break;
            }
        }
        if matching {
            return Some(i);
        }
    }
    None
}

pub struct Spliterator<'a, T: Eq> {
    pub string: &'a [T],
    pub finished: bool,
    pub seq: &'a [T],
}

impl<'a, T: Eq> Spliterator<'a, T> {
    pub fn new(string: &'a [T], seq: &'a [T]) -> Self {
        Self {
            string,
            finished: false,
            seq,
        }
    }

    pub fn find_seq(&self) -> Option<usize> {
        find_in_slice(self.string, self.seq)
    }
}

impl<'a, T: Eq> Iterator for Spliterator<'a, T> {
    type Item = &'a [T];

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        match self.find_seq() {
            Some(v) => {
                let (ret, rest) = self.string.split_at(v);
                self.string = &rest[self.seq.len()..];
                Some(ret)
            }
            None => {
                self.finished = true;
                Some(self.string)
            }
        }
    }
}

impl<'a, T: Eq> FusedIterator for Spliterator<'a, T> {}

#[derive(Debug, Clone, Default)]
pub struct AsyncReader<T: Read> {
    pub reader: T,
}

impl<T: Read + Unpin> Deref for AsyncReader<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}

impl<T: Read + Unpin> AsyncRead for AsyncReader<T> {
    fn poll_read(mut self: Pin<&mut Self>, _: &mut Context, buf: &mut [u8]) -> Poll<Result<usize>> {
        Poll::Ready(self.reader.read(buf))
    }
}

impl<T: Read + Unpin> AsyncReader<T> {
    pub fn new(reader: T) -> Self {
        Self { reader }
    }
}
