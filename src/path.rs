use std::collections::HashMap;
use std::convert::TryFrom;
use std::ops::Deref;

#[derive(Debug, Clone, PartialEq, Eq)]
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
