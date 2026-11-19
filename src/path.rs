use crate::decode::unescape;
use crate::{default, deref};
use std::convert::TryFrom;
use std::error;
use std::fmt;
use std::iter::{FromIterator, Iterator};
use std::path::PathBuf as StdPathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Custom error type for invalid paths
pub struct InvalidPathError;

impl fmt::Display for InvalidPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        // TODO: make more informative
        write!(f, "Invalid path error")
    }
}

impl error::Error for InvalidPathError {}

// Custom PathBuf for managing custom url queries and variables,
// Provides methods to match and check on urls
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathBuf {
    pub chunks: Vec<String>,
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

    pub fn to_std_pathbuf(&self) -> StdPathBuf {
        let mut path_string = String::new();
        for chunks in &self.chunks {
            path_string.push_str(format!("{}/", chunks).as_str());
        }
        StdPathBuf::from(path_string)
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

    /// Parse the path of a request. Like [`parse`](Self::parse), but every
    /// segment is percent-decoded first (a `+` stays a `+`), so routes and
    /// url variables see `/users/Ada%20Lovelace` as `users`, `Ada Lovelace`.
    /// The path is split before decoding: an encoded `/` (`%2F`) stays
    /// inside its segment. A segment that decodes to `.` or `..` counts as
    /// one, so `%2e%2e` climbs a level like `..` does instead of reaching a
    /// handler as a variable.
    pub fn parse_request(path: &str) -> Result<Self, InvalidPathError> {
        let mut chunks = Vec::new();
        for chunk in path.split('/') {
            if chunk.is_empty() {
                continue;
            }
            let chunk = unescape(chunk, false);
            match &*chunk {
                "." => {}
                ".." => {
                    if chunks.pop().is_none() {
                        return Err(InvalidPathError);
                    }
                }
                _ => chunks.push(chunk.into_owned()),
            }
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
    /// Check if a Pathbuf matches the given pathbuf in url
    #[cfg(feature = "url_variables")]
    pub fn check_matches<'a>(&'a self, other: &'a PathBuf) -> Option<Vars<'a>> {
        if self.len() != other.len() {
            return None;
        }
        let mut vars = Vars::new();
        for (a, b) in self.iter().zip(other.iter()) {
            if a.as_bytes()[0] == b':' {
                vars.vars.push((&a[1..], &b[..]));
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

/// The url variables of a matched route, like `id` in `/users/:id`, in the
/// order they appear in the route. They are borrowed from the request path.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Vars<'a> {
    vars: Vec<(&'a str, &'a str)>,
}

impl<'a> Vars<'a> {
    pub fn new() -> Self {
        Self { vars: Vec::new() }
    }
    /// The value of the variable `name`
    pub fn get(&self, name: &str) -> Option<&'a str> {
        self.vars
            .iter()
            .find(|(var, _)| *var == name)
            .map(|(_, value)| *value)
    }
    /// Every variable and its value
    pub fn iter(&self) -> impl Iterator<Item = (&'a str, &'a str)> + '_ {
        self.vars.iter().copied()
    }
    pub fn len(&self) -> usize {
        self.vars.len()
    }
    pub fn is_empty(&self) -> bool {
        self.vars.is_empty()
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
    pub vars: Vars<'a>,
    pub data: &'a T,
}

/// A trie of paths, one level per segment
#[derive(Debug, Clone)]
pub struct PathNode<T> {
    // literal segments, sorted so the request's segment is found by binary
    // search, without hashing or copying it
    chunks: Vec<(String, PathNode<T>)>,
    // a `:variable` segment
    var: Option<Box<PathNode<T>>>,
    // what was inserted at exactly this path
    end: Vec<PathData<T>>,
}

impl<T> PathNode<T> {
    pub fn new() -> Self {
        PathNode {
            chunks: Vec::new(),
            var: None,
            end: Vec::new(),
        }
    }

    pub fn insert(&mut self, path: PathBuf, data: T) {
        let mut cur = self;
        for chunk in path.chunks.iter() {
            // segments are never empty, `PathBuf::parse` drops those
            cur = if chunk.as_bytes()[0] == b':' {
                &mut **cur.var.get_or_insert_with(|| Box::new(PathNode::new()))
            } else {
                let at = match cur
                    .chunks
                    .binary_search_by(|(key, _)| key.as_str().cmp(chunk))
                {
                    Ok(at) => at,
                    Err(at) => {
                        cur.chunks.insert(at, (chunk.clone(), PathNode::new()));
                        at
                    }
                };
                &mut cur.chunks[at].1
            };
        }
        cur.end.push(PathData {
            orig_path: path,
            data,
        });
    }

    /// Hand everything inserted at a path that matches `path` to `found`,
    /// with the url variables of the match
    pub(crate) fn find<'a, F>(&'a self, path: &'a PathBuf, found: &mut F)
    where
        F: FnMut(MatchedPath<'a, T>),
    {
        self.find_from(&path.chunks, path, found);
    }

    fn find_from<'a, F>(&'a self, rest: &[String], path: &'a PathBuf, found: &mut F)
    where
        F: FnMut(MatchedPath<'a, T>),
    {
        match rest.split_first() {
            None => {
                for data in &self.end {
                    found(MatchedPath {
                        #[cfg(feature = "url_variables")]
                        vars: data.orig_path.check_matches(path).unwrap_or_default(),
                        data: &data.data,
                    });
                }
            }
            Some((first, rest)) => {
                let at = self
                    .chunks
                    .binary_search_by(|(key, _)| key.as_str().cmp(first));
                if let Ok(at) = at {
                    self.chunks[at].1.find_from(rest, path, found);
                }
                #[cfg(feature = "url_variables")]
                {
                    if let Some(node) = &self.var {
                        node.find_from(rest, path, found);
                    }
                }
            }
        }
    }

    /// Whether anything was inserted at a path that matches `path`
    pub(crate) fn matches(&self, path: &PathBuf) -> bool {
        let mut found = false;
        self.find(path, &mut |_| found = true);
        found
    }

    /// Everything inserted at a path that matches `path`
    #[cfg(test)]
    pub fn get<'a>(&'a self, path: &'a PathBuf) -> Vec<MatchedPath<'a, T>> {
        let mut matched = Vec::new();
        self.find(path, &mut |m| matched.push(m));
        matched
    }

    // Move everything in the trie into `all`
    fn drain_into(self, all: &mut Vec<PathData<T>>) {
        all.extend(self.end);
        for (_, node) in self.chunks {
            node.drain_into(all);
        }
        if let Some(node) = self.var {
            node.drain_into(all);
        }
    }
}

impl<T> IntoIterator for PathNode<T> {
    type Item = PathData<T>;
    type IntoIter = std::vec::IntoIter<PathData<T>>;

    fn into_iter(self) -> Self::IntoIter {
        let mut all = Vec::new();
        self.drain_into(&mut all);
        all.into_iter()
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

// CTL from RFC 5234: %x00-1F / %x7F
// only the tokio request parser uses it
#[cfg_attr(not(feature = "std"), allow(dead_code))]
pub fn is_ctl(c: char) -> bool {
    c <= '\x1f' || c == '\x7f'
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn success_standard() {
        // Parsing should work as expected.
        let chunks = PathBuf::parse("/asdf//foo/a/b/c/../d").unwrap();
        assert_eq!(*chunks.chunks(), vec!["asdf", "foo", "a", "b", "d"]);
    }

    #[test]
    #[cfg(feature = "url_variables")]
    fn success_matching() {
        // Parsing should work as expected.
        let path1 = PathBuf::parse("/asdf/:var/foo/").unwrap();
        let path2 = PathBuf::parse("asdf/test/foo/").unwrap();
        let path3 = PathBuf::parse("/asdf/test/foo").unwrap();
        let path4 = PathBuf::parse("/asdf/test/bad").unwrap();
        assert_eq!(
            path1.check_matches(&path2).unwrap().get("var").unwrap(),
            "test".to_string()
        );
        assert_eq!(
            path1.check_matches(&path3).unwrap().get("var").unwrap(),
            "test".to_string()
        );
        assert!(path1.check_matches(&path4).is_none());
    }

    #[test]
    #[cfg(not(feature = "url_variables"))]
    fn success_matching() {
        // Parsing should work as expected.
        let path1 = PathBuf::parse("/asdf/test/foo/").unwrap();
        let path2 = PathBuf::parse("asdf/test/foo").unwrap();
        let path3 = PathBuf::parse("asdf/test/bad").unwrap();
        path1.check_matches(&path2).unwrap();
        assert!(path1.check_matches(&path3).is_none());
    }

    #[test]
    fn success_subtraction() {
        // Parsing should work as expected.
        let path1 = PathBuf::parse("/a/b/c/d/").unwrap();
        let path2 = PathBuf::parse("/a/b/").unwrap();
        let path3 = PathBuf::parse("/c/d").unwrap();
        let path4 = PathBuf::parse("a/:x/c").unwrap();
        assert_eq!(*path1.subtract(&path2).unwrap().chunks(), vec!["c", "d"]);
        assert!(path1.subtract(&path3).is_none());
        assert!(path1.subtract(&path4).is_none());
    }

    #[test]
    #[should_panic]
    fn fail_traversal() {
        // Too many ..s should error.
        PathBuf::parse("/asdf/../..").unwrap();
    }

    #[test]
    fn request_paths_are_decoded() {
        let path = PathBuf::parse_request("/users/Ada%20Lovelace/caf%C3%A9").unwrap();
        assert_eq!(*path.chunks(), vec!["users", "Ada Lovelace", "café"]);
        // a `+` is only a space in queries
        let path = PathBuf::parse_request("/a+b").unwrap();
        assert_eq!(*path.chunks(), vec!["a+b"]);
        // split first, an encoded slash is part of the segment
        let path = PathBuf::parse_request("/files/a%2Fb").unwrap();
        assert_eq!(*path.chunks(), vec!["files", "a/b"]);
    }

    #[test]
    fn encoded_dot_segments_are_dot_segments() {
        let path = PathBuf::parse_request("/static/%2e%2E/a/%2e/b").unwrap();
        assert_eq!(*path.chunks(), vec!["a", "b"]);
        assert!(PathBuf::parse_request("/%2e%2e").is_err());
        // only a whole segment is a dot segment
        let path = PathBuf::parse_request("/%2e%2e%2Fetc").unwrap();
        assert_eq!(*path.chunks(), vec!["../etc"]);
    }

    #[test]
    #[cfg(feature = "url_variables")]
    fn route_paths_are_not_decoded() {
        let route = PathBuf::parse("/users/:name").unwrap();
        let request = PathBuf::parse_request("/users/Ada%20Lovelace").unwrap();
        let vars = route.check_matches(&request).unwrap();
        assert_eq!(vars.get("name"), Some("Ada Lovelace"));
        assert_eq!(*PathBuf::parse("/a%20b").unwrap().chunks(), vec!["a%20b"]);
    }

    #[test]
    #[cfg(feature = "url_variables")]
    fn success_tree() {
        let mut temp_node: PathNode<i32> = PathNode::new();
        let path1 = PathBuf::parse("/asdf/:var/foo/").unwrap();
        let path2 = PathBuf::parse("asdf/test/foo/").unwrap();
        let path3 = PathBuf::parse("/asdf/test/foo").unwrap();
        let path4 = PathBuf::parse("/asdf/test/bad").unwrap();
        let path5 = PathBuf::parse("/asdf/test/nope").unwrap();
        temp_node.insert(path1, 1);
        temp_node.insert(path4.clone(), 4);
        let mut node: PathNode<i32> = PathNode::new();
        node.extend(temp_node);
        assert!(node.get(&path5).is_empty());
        let matched = node.get(&path2).remove(0);
        assert_eq!(*matched.data, 1);
        assert_eq!(matched.vars.get("var").unwrap(), "test");
        let matched = node.get(&path3).remove(0);
        assert_eq!(*matched.data, 1);
        assert_eq!(matched.vars.get("var").unwrap(), "test");
        let matched = node.get(&path4).remove(0);
        assert_eq!(*matched.data, 4);
        assert!(matched.vars.is_empty());
    }

    #[test]
    #[cfg(not(feature = "url_variables"))]
    fn success_tree() {
        let mut node: PathNode<i32> = PathNode::new();
        let path1 = PathBuf::parse("asdf/test/foo/").unwrap();
        let path2 = PathBuf::parse("/asdf/test/bad").unwrap();
        node.insert(path1.clone(), 1);
        assert!(node.get(&path2).is_empty());
        let matched = node.get(&path1).remove(0);
        assert_eq!(*matched.data, 1);
    }
}
