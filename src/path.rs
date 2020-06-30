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
    pub fn check_matches(&self, other: &PathBuf) -> Option<HashMap<String, String>> {
        if self.len() != other.len() {
            return None;
        }
        let mut vars = HashMap::new();
        for (a, b) in self.iter().zip(other.iter()) {
            if a.as_bytes()[0] == b':' {
                vars.insert(a[1..].to_owned(), b.clone());
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
    pub vars: Vec<String>,
    pub data: T,
}

#[derive(Debug, Clone)]
pub struct MatchedPath<'a, T> {
    #[cfg(feature = "url_variables")]
    pub vars: HashMap<&'a String, String>,
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
    Leaf(PathData<T>),
}

impl<T: std::fmt::Debug> PathNode<T> {
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

    fn unwrap_leaf(&self) -> &PathData<T> {
        if let PathNode::Leaf(x) = self {
            x
        } else {
            panic!("Tried to unwrap a node as a leaf.");
        }
    }

    pub fn new() -> Self {
        PathNode::Node(HashMap::new())
    }

    pub fn insert(&mut self, path: PathBuf, data: T) -> Result<(), &'static str> {
        let mut cur = self.unwrap_node_mut();
        #[cfg(feature = "url_variables")]
        let mut vars = Vec::new();
        for chunk in path.chunks.into_iter() {
            #[cfg(feature = "url_variables")]
            if chunk.as_bytes()[0] == b':' {
                vars.push(chunk[1..].to_owned());
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
        // right now this overwrites the current value
        // this may be changed later for consistency
        cur.insert(
            PathChunk::End,
            PathNode::Leaf(PathData {
                #[cfg(feature = "url_variables")]
                vars,
                data,
            }),
        );
        Ok(())
    }

    fn dfs<'a, 'b: 'a>(
        &self,
        chunks: &'b [String],
        #[cfg(feature = "url_variables")] var_chunks: &mut Vec<&'a String>,
    ) -> Option<&PathNode<T>> {
        let cur = self.unwrap_node();
        if chunks.is_empty() {
            return cur.get(&PathChunk::End);
        }
        if let Some(v) = cur.get(&PathChunk::Chunk(chunks[0].clone())) {
            let found = v.dfs(
                &chunks[1..],
                #[cfg(feature = "url_variables")]
                var_chunks,
            );
            if found.is_some() {
                return found;
            }
        }
        #[cfg(feature = "url_variables")]
        {
            var_chunks.push(&chunks[0]);
            if let Some(v) = cur.get(&PathChunk::CatchAll) {
                let found = v.dfs(&chunks[1..], var_chunks);
                if found.is_some() {
                    return found;
                }
            }
            var_chunks.pop();
        }
        None
    }

    pub fn get(&self, path: &PathBuf) -> Option<MatchedPath<T>> {
        #[cfg(feature = "url_variables")]
        let mut var_chunks = Vec::new();
        let data = self
            .dfs(
                path.chunks.as_slice(),
                #[cfg(feature = "url_variables")]
                &mut var_chunks,
            )?
            .unwrap_leaf();
        #[cfg(feature = "url_variables")]
        let mut vars = HashMap::new();
        #[cfg(feature = "url_variables")]
        if data.vars.len() != var_chunks.len() {
            panic!("PathNode structure is corrupted.");
        }
        #[cfg(feature = "url_variables")]
        for (n, v) in data.vars.iter().zip(var_chunks.into_iter()) {
            vars.insert(n, v.clone());
        }
        Some(MatchedPath {
            #[cfg(feature = "url_variables")]
            vars,
            data: &data.data,
        })
    }
}

impl<T: std::fmt::Debug> Default for PathNode<T> {
    fn default() -> Self {
        Self::new()
    }
}
