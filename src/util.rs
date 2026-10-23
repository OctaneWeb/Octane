use octane_http::http1x::find_in_slice;
use std::iter::FusedIterator;

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
