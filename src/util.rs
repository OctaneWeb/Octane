use std::io::Read;
use std::iter::FusedIterator;
use std::mem;
use std::ops::Deref;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, Result};

pub fn from_hex(chr: char) -> Option<u8> {
    if chr > 'f' {
        return None;
    }
    let c = chr as u8;
    if c >= b'0' && c <= b'9' {
        return Some(c - b'0');
    }
    if c >= b'A' && c <= b'F' {
        return Some(c - b'A' + 10);
    }
    if c >= b'a' && c <= b'f' {
        return Some(c - b'a' + 10);
    }
    None
}

pub struct DoublePeek<'a, T>
where
    T: Default,
{
    pub iter: &'a mut dyn Iterator<Item = T>,
    pub cache: [T; 2],
    pub stored: usize,
    pub unpeek: bool,
}

impl<'a, T> DoublePeek<'a, T>
where
    T: Default,
{
    pub fn new(it: &'a mut dyn Iterator<Item = T>) -> Self {
        let cache: [T; 2] = Default::default();
        Self {
            iter: it,
            cache,
            stored: 0,
            unpeek: false,
        }
    }

    pub fn peek(&mut self) -> Option<&T> {
        if self.unpeek && self.stored > 0 {
            self.unpeek = false;
            return Some(&self.cache[0]);
        }
        self.unpeek = false;
        if self.stored == 2 {
            return None;
        }
        let item = self.iter.next();
        match item {
            None => None,
            Some(v) => {
                self.cache[self.stored] = v;
                self.stored += 1;
                Some(&self.cache[self.stored - 1])
            }
        }
    }
}

impl<'a, T> Iterator for DoublePeek<'a, T>
where
    T: Default,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.stored == 1 {
            self.stored = 0;
            return Some(mem::take(&mut self.cache[0]));
        } else if self.stored == 2 {
            self.stored = 1;
            let dat1 = mem::take(&mut self.cache[1]);
            return Some(mem::replace(&mut self.cache[0], dat1));
        }
        self.iter.next()
    }
}

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

    pub fn skip_empty(&mut self) {
        while let Some(0) = self.find_seq() {
            self.next();
        }
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
