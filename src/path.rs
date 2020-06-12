use std::convert::TryFrom;
use std::ops::Deref;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathBuf {
    pub chunks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidPathError {}

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
                if let None = chunks.pop() {
                    return Err(InvalidPathError {});
                }
                continue;
            }
            chunks.push(chunk.to_owned());
        }
        Ok(PathBuf { chunks })
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
