use std::collections::HashMap;
use std::convert::TryFrom;
use std::ops::Deref;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathBuf {
    pub chunks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidPathError;

impl PathBuf {
    pub fn new() -> Self {
        PathBuf { chunks: Vec::new() }
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
}

impl Deref for PathBuf {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.chunks
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

impl Default for PathBuf {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathData<T> {
    #[cfg(feature = "url_variables")]
    pub orig_path: PathBuf,
    pub data: T,
}

#[derive(Debug, Clone)]
pub struct MatchedPath<'a, T> {
    #[cfg(feature = "url_variables")]
    pub vars: HashMap<&'a str, String>,
    pub data: &'a T,
}

impl<T> Deref for PathData<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<'a, T> Deref for MatchedPath<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
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
        #[cfg(feature = "url_variables")]
        {
            path_chunks = path.clone().chunks;
        }
        #[cfg(not(feature = "url_variables"))]
        {
            path_chunks = path.chunks;
        }
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
            .or_insert(PathNode::Leaf(vec![]))
            .unwrap_leaf_mut()
            .push(PathData {
                #[cfg(feature = "url_variables")]
                orig_path: path,
                data,
            });
    }

    fn dfs<'a, 'b: 'a>(&self, chunks: &'b [String]) -> Vec<&PathData<T>> {
        let cur = self.unwrap_node();
        if chunks.is_empty() {
            return cur
                .get(&PathChunk::End)
                .map(|v| v.unwrap_leaf().iter().collect())
                .unwrap_or(vec![]);
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

    pub fn get(&self, path: &PathBuf) -> Vec<MatchedPath<T>> {
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
                    .map(|(k, v)| (k, v.to_owned()))
                    .collect(),
                data: &data.data,
            })
            .collect()
    }
}

impl<T> Default for PathNode<T> {
    fn default() -> Self {
        Self::new()
    }
}
