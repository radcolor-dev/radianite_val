use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicCacheContext {
    root: PathBuf,
    app_version: String,
}

impl PublicCacheContext {
    pub fn new(root: PathBuf, app_version: String) -> Self {
        Self { root, app_version }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn app_version(&self) -> &str {
        &self.app_version
    }
}
