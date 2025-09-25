use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

#[derive(Clone)]
pub struct IgnoreMatcher {
    root: PathBuf,
    set: GlobSet,
}

impl IgnoreMatcher {
    pub fn new(root: &Path, patterns: &[String]) -> Result<Self> {
        let mut builder = GlobSetBuilder::new();
        // Default ignores to avoid feedback loops and OS artifacts.
        for pattern in [
            ".git",
            ".git/**",
            ".gitignore",
            "**/.DS_Store",
            "**/Thumbs.db",
        ] {
            let glob = GlobBuilder::new(pattern)
                .literal_separator(true)
                .build()
                .with_context(|| format!("failed to compile builtin ignore pattern '{pattern}'"))?;
            builder.add(glob);
        }

        for pattern in patterns {
            if pattern.trim().is_empty() {
                continue;
            }
            let glob = GlobBuilder::new(pattern)
                .literal_separator(false)
                .build()
                .with_context(|| format!("failed to compile ignore pattern '{pattern}'"))?;
            builder.add(glob);
        }

        let set = builder.build().context("failed to build ignore set")?;
        Ok(Self {
            root: root.to_path_buf(),
            set,
        })
    }

    pub fn should_ignore<P: AsRef<Path>>(&self, path: P) -> bool {
        let path = path.as_ref();
        if let Ok(rel) = path.strip_prefix(&self.root) {
            if rel.as_os_str().is_empty() {
                return false;
            }
            if let Some(rel_str) = rel.to_str() {
                let normalized = rel_str.replace('\\', "/");
                return self.set.is_match(normalized.as_str());
            }
        }
        false
    }
}
