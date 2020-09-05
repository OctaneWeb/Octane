use crate::error::InvalidPathError;
use crate::{default, deref};
use std::collections::{hash_map, HashMap};
use std::convert::TryFrom;
use std::fmt;
use std::iter::{Iterator, Map, FromIterator};
use std::path::PathBuf as StdPathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathBuf {
    chunks: Vec<String>,
}

impl fmt::Display for PathBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let mut path_string = String::new();
        for chunks in self.chunks.iter() {
            path_string.push_str(format!("{}/", chunks).as_str());
        }
        write!(f, "{}", path_string)
    }
}

impl PathBuf {
    pub fn new() -> Self {
        PathBuf { chunks: Vec::new() }
    }
    pub fn chunks(&self) -> &Vec<String> {
        self.chunks.as_ref()
    }

    pub fn to_std_pathbuf(&self) -> std::path::PathBuf {
        let mut path_string = String::new();
        for chunks in &self.chunks {
            path_string.push_str(format!("{}/", chunks).as_str());
        }
        std::path::PathBuf::from(path_string)
    }

    pub fn parse(path: &str) -> Result<Self, InvalidPathError> {
        let mut chunks = Vec::new();
        for chunk in path.split('/') {
            if chunk.is_empty() || chunk == "." {
                continue;
            }
            if chunk == ".." {
                if chunks.pop().is_none() {
                    return Err(InvalidPathError);
                }
                continue;
            }
            chunks.push(chunk.to_owned());
        }
        Ok(PathBuf { chunks })
    }

    pub fn check_starts_with(&self, other: &PathBuf) -> bool {
        for (a, b) in self.iter().zip(other.iter()) {
            if a != b {
                return false;
            }
        }
        true
    }

    #[cfg(feature = "url_variables")]
    pub fn check_matches<'a, 'b>(
        &'a self,
        other: &'b PathBuf,
    ) -> Option<HashMap<&'a str, &'b str>> {
        if self.len() != other.len() {
            return None;
        }
        let mut vars = HashMap::new();
        for (a, b) in self.iter().zip(other.iter()) {
            if a.as_bytes()[0] == b':' {
                vars.insert(&a[1..], &b[..]);
            } else if a != b {
                return None;
            }
        }
        Some(vars)
    }

    #[cfg(not(feature = "url_variables"))]
    pub fn check_matches(&self, other: &PathBuf) -> Option<()> {
        if self.iter().ne(other.iter()) {
            return None;
        }
        Some(())
    }

    pub fn subtract(&self, other: &PathBuf) -> Option<PathBuf> {
        if self.len() < other.len() {
            return None;
        }
        if self.iter().take(other.len()).ne(other.iter()) {
            return None;
        }
        Some(PathBuf {
            chunks: self.chunks[other.len()..].to_vec(),
        })
    }

    pub fn concat_owned(&self, other: PathBuf) -> PathBuf {
        PathBuf {
            chunks: self
                .chunks
                .iter()
                .cloned()
                .chain(other.chunks.into_iter())
                .collect(),
        }
    }
    pub fn concat(&self, other: &PathBuf) -> PathBuf {
        PathBuf {
            chunks: self
                .chunks
                .iter()
                .cloned()
                .chain(other.chunks.iter().cloned())
                .collect(),
        }
    }
}

impl From<PathBuf> for StdPathBuf {
    fn from(p: PathBuf) -> StdPathBuf {
        p.chunks.into_iter().collect()
    }
}

impl FromStr for PathBuf {
    type Err = InvalidPathError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathData<T> {
    pub orig_path: PathBuf,
    pub data: T,
}

#[derive(Debug, Clone)]
pub struct MatchedPath<'a, T> {
    #[cfg(feature = "url_variables")]
    pub vars: HashMap<&'a str, &'a str>,
    pub data: &'a T,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathChunk {
    Chunk(String),
    CatchAll,
    End,
}

#[derive(Debug, Clone)]
pub enum PathNode<T> {
    Node(HashMap<PathChunk, PathNode<T>>),
    Leaf(Vec<PathData<T>>),
}

impl<T> PathNode<T> {
    fn unwrap_node(&self) -> &HashMap<PathChunk, PathNode<T>> {
        if let PathNode::Node(x) = self {
            x
        } else {
            panic!("Tried to unwrap a leaf as a node.");
        }
    }

    fn unwrap_node_mut(&mut self) -> &mut HashMap<PathChunk, PathNode<T>> {
        if let PathNode::Node(x) = self {
            x
        } else {
            panic!("Tried to unwrap a leaf as a node.");
        }
    }

    fn unwrap_leaf(&self) -> &Vec<PathData<T>> {
        if let PathNode::Leaf(x) = self {
            x
        } else {
            panic!("Tried to unwrap a node as a leaf.");
        }
    }

    fn unwrap_leaf_mut(&mut self) -> &mut Vec<PathData<T>> {
        if let PathNode::Leaf(x) = self {
            x
        } else {
            panic!("Tried to unwrap a node as a leaf.");
        }
    }

    pub fn new() -> Self {
        PathNode::Node(HashMap::new())
    }

    pub fn insert(&mut self, path: PathBuf, data: T) {
        let mut cur = self.unwrap_node_mut();
        let path_chunks;
        path_chunks = path.clone().chunks;
        for chunk in path_chunks.into_iter() {
            if chunk.as_bytes()[0] == b':' {
                cur = cur
                    .entry(PathChunk::CatchAll)
                    .or_default()
                    .unwrap_node_mut();
                continue;
            }
            cur = cur
                .entry(PathChunk::Chunk(chunk))
                .or_default()
                .unwrap_node_mut();
        }
        cur.entry(PathChunk::End)
            .or_insert_with(|| PathNode::Leaf(vec![]))
            .unwrap_leaf_mut()
            .push(PathData {
                orig_path: path,
                data,
            });
    }

    fn dfs(&self, chunks: &[String]) -> Vec<&PathData<T>> {
        let cur = self.unwrap_node();
        if chunks.is_empty() {
            return cur
                .get(&PathChunk::End)
                .map(|v| v.unwrap_leaf().iter().collect())
                .unwrap_or_default();
        }
        let mut ret = vec![];
        if let Some(v) = cur.get(&PathChunk::Chunk(chunks[0].clone())) {
            let found = v.dfs(&chunks[1..]);
            ret = found;
        }
        #[cfg(feature = "url_variables")]
        {
            if let Some(v) = cur.get(&PathChunk::CatchAll) {
                let found = v.dfs(&chunks[1..]);
                ret.extend(found);
            }
        }
        ret
    }

    pub fn get<'a>(&'a self, path: &'a PathBuf) -> Vec<MatchedPath<'a, T>> {
        let matched = self.dfs(path.chunks.as_slice());
        matched
            .into_iter()
            .map(|data| MatchedPath {
                #[cfg(feature = "url_variables")]
                vars: data
                    .orig_path
                    .check_matches(path)
                    .unwrap()
                    .into_iter()
                    .map(|(k, v)| (k, v))
                    .collect(),
                data: &data.data,
            })
            .collect()
    }
}

pub struct PathNodeIterator<'a, T> {
    stack: Vec<hash_map::Values<'a, PathChunk, PathNode<T>>>,
    curvec: Option<&'a Vec<PathData<T>>>,
    curind: usize,
}

pub struct OwnedPathNodeIterator<T, F>
where
    F: FnMut((PathChunk, PathNode<T>)) -> PathNode<T>,
{
    stack: Vec<Map<hash_map::IntoIter<PathChunk, PathNode<T>>, F>>,
    curvec: Option<Vec<PathData<T>>>,
}

impl<'a, T> Iterator for PathNodeIterator<'a, T> {
    type Item = &'a PathData<T>;

    fn next(&mut self) -> Option<Self::Item> {
        // loop due to lack of tail recursive optimizations
        loop {
            if let Some(v) = self.curvec {
                if self.curind < v.len() {
                    let ret = &v[self.curind];
                    self.curind += 1;
                    return Some(ret);
                }
            }
            self.curind = 0;
            self.curvec = None;
            if self.stack.is_empty() {
                return None;
            }
            let len = self.stack.len();
            if let Some(v) = self.stack[len - 1].next() {
                // using continue as tail recursion
                match v {
                    PathNode::Node(n) => {
                        self.stack.push(n.values());
                        continue;
                    }
                    PathNode::Leaf(l) => {
                        self.curvec = Some(l);
                        continue;
                    }
                }
            } else {
                self.stack.pop();
                continue;
            }
        }
    }
}

impl<T> PathNode<T> {
    pub fn iter(&self) -> PathNodeIterator<T> {
        match self {
            PathNode::Node(n) => PathNodeIterator {
                stack: vec![n.values()],
                curvec: None,
                curind: 0,
            },
            PathNode::Leaf(l) => PathNodeIterator {
                stack: vec![],
                curvec: Some(l),
                curind: 0,
            },
        }
    }
}

impl<T> Iterator for OwnedPathNodeIterator<T, fn((PathChunk, PathNode<T>)) -> PathNode<T>> {
    type Item = PathData<T>;

    fn next(&mut self) -> Option<Self::Item> {
        // loop due to lack of tail recursive optimizations
        loop {
            if let Some(v) = &mut self.curvec {
                if !v.is_empty() {
                    return v.pop();
                }
            }
            self.curvec = None;
            if self.stack.is_empty() {
                return None;
            }
            let len = self.stack.len();
            if let Some(v) = self.stack[len - 1].next() {
                // using continue as tail recursion
                match v {
                    PathNode::Node(n) => {
                        self.stack.push(n.into_iter().map(get_second));
                        continue;
                    }
                    PathNode::Leaf(l) => {
                        self.curvec = Some(l);
                        continue;
                    }
                }
            } else {
                self.stack.pop();
                continue;
            }
        }
    }
}

impl<T> IntoIterator for PathNode<T> {
    type Item = PathData<T>;
    type IntoIter = OwnedPathNodeIterator<T, fn((PathChunk, PathNode<T>)) -> PathNode<T>>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            PathNode::Node(n) => OwnedPathNodeIterator {
                stack: vec![n.into_iter().map(get_second)],
                curvec: None,
            },
            PathNode::Leaf(l) => OwnedPathNodeIterator {
                stack: vec![],
                curvec: Some(l),
            },
        }
    }
}

impl<T> Extend<PathData<T>> for PathNode<T> {
    fn extend<I: IntoIterator<Item = PathData<T>>>(&mut self, iter: I) {
        for dat in iter {
            self.insert(dat.orig_path, dat.data);
        }
    }
}

impl<T> FromIterator<PathData<T>> for PathNode<T> {
    fn from_iter<I: IntoIterator<Item = PathData<T>>>(iter: I) -> Self {
        let mut ret = Self::new();
        for dat in iter {
            ret.insert(dat.orig_path, dat.data);
        }
        ret
    }
}

impl TryFrom<String> for PathBuf {
    type Error = InvalidPathError;

    fn try_from(val: String) -> Result<Self, Self::Error> {
        Self::parse(&val)
    }
}

impl TryFrom<&str> for PathBuf {
    type Error = InvalidPathError;

    fn try_from(val: &str) -> Result<Self, Self::Error> {
        Self::parse(val)
    }
}

default!(PathBuf);
default!(PathNode<T>);
deref!(PathData<T>, T, data);
deref!(MatchedPath<'a, T>, T, data);
deref!(PathBuf, Vec<String>, chunks);

pub fn is_ctl(c: char) -> bool {
    c < '\x1f' || c == '\x7f'
}

fn get_second<T>(tup: (PathChunk, PathNode<T>)) -> PathNode<T> {
    tup.1
}
