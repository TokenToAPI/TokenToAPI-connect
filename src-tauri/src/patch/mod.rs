// Adapted from CC Switch, MIT. Removed full-file engine/DB error adapters.
//! Order-preserving patchers: they start from the existing file and only change the
//! named keys, leaving every other key, value and ordering untouched.
//!
//! Each format has its own patch type, all implementing [`LivePatch`]: they take the
//! bytes before the write (`None` when the file does not exist) and return the bytes
//! after. Unparseable input is an error and never falls back to an empty document
//! (C0 incidents and v3.11.0 wrote from an empty document after a parse failure,
//! which wiped user configuration).

pub mod json;
pub mod toml;

use std::fmt;
use std::path::{Path, PathBuf};

/// A key location in the document: a key sequence starting from the root. Keys may
/// contain dots (TOML `model."grok-4.5"`), so dotted strings are not used.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyPath(pub Vec<String>);

impl KeyPath {
    pub fn new(segments: &[&str]) -> Self {
        Self(segments.iter().map(|s| (*s).to_string()).collect())
    }

    pub fn root() -> Self {
        Self(Vec::new())
    }

    pub fn child(&self, key: &str) -> Self {
        let mut segments = self.0.clone();
        segments.push(key.to_string());
        Self(segments)
    }

    fn split_last(&self) -> Option<(&[String], &String)> {
        self.0.split_last().map(|(last, parent)| (parent, last))
    }
}

impl fmt::Display for KeyPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return f.write_str("<root>");
        }
        f.write_str(&self.0.join("."))
    }
}

/// Computes the new content of a file in memory.
pub trait LivePatch {
    fn apply(&self, path: &Path, pre: Option<&[u8]>) -> Result<Vec<u8>, LiveWriteError>;

    /// Entry point used by the engine: `None` means the file is deleted (for
    /// example `auth.json` when Codex switches to a third party). By default this
    /// always writes the result of [`apply`](LivePatch::apply).
    fn apply_file(
        &self,
        path: &Path,
        pre: Option<&[u8]>,
    ) -> Result<Option<Vec<u8>>, LiveWriteError> {
        self.apply(path, pre).map(Some)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LiveWriteError {
    /// The file could not be parsed. Line and column numbers start at 1.
    #[error("Could not parse {path} (line {line}, column {column}): {message}")]
    Parse {
        path: PathBuf,
        line: usize,
        column: usize,
        message: String,
    },
    /// Parseable, but the location to change has an unexpected shape, for example
    /// `env` being a string instead of an object.
    #[error("{key_path} in {path} is not {expected}; nothing was written to avoid overwriting your configuration")]
    Shape {
        path: PathBuf,
        key_path: KeyPath,
        expected: &'static str,
    },
    #[error("I/O error: {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// The file kept changing across several re-reads, so the write was abandoned.
    #[error("{path} kept being modified by another program during the write; nothing was written")]
    Conflict { path: PathBuf },
    /// After the editor opened the file, these keys were changed by another program,
    /// conflicting with the editor changes.
    #[error("{keys:?} in {path} were modified by another program while editing")]
    EditConflict { path: PathBuf, keys: Vec<String> },
    /// After the write, the route actually used by the client is not the target
    /// provider: the currently active profile overrides the routing.
    #[error("Codex’s currently active profile \"{profile}\" overrides {key}")]
    Route { profile: String, key: String },
}

/// Byte offset -> (line, column), both starting at 1.
pub(crate) fn line_column(text: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(text.len());
    let before = &text[..offset];
    let line = before.matches('\n').count() + 1;
    let column = before.rfind('\n').map_or(before.chars().count(), |nl| {
        before[nl + 1..].chars().count()
    }) + 1;
    (line, column)
}

pub(crate) fn decode_utf8<'a>(path: &Path, bytes: &'a [u8]) -> Result<&'a str, LiveWriteError> {
    std::str::from_utf8(bytes).map_err(|err| {
        let (line, column) = line_column(
            // The part before valid_up_to is guaranteed to be valid UTF-8.
            std::str::from_utf8(&bytes[..err.valid_up_to()]).unwrap_or_default(),
            err.valid_up_to(),
        );
        LiveWriteError::Parse {
            path: path.to_path_buf(),
            line,
            column,
            message: "not UTF-8 text".to_string(),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_column_counts_from_one() {
        let text = "ab\ncde\nf";
        assert_eq!(line_column(text, 0), (1, 1));
        assert_eq!(line_column(text, 4), (2, 2));
        assert_eq!(line_column(text, text.len()), (3, 2));
    }
}
