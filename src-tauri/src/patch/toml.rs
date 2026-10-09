// Vendored from CC Switch, MIT. See licenses/CC-Switch-MIT.txt and UPSTREAM.md.
//! TOML patcher (Codex and Grok Build `config.toml`), built on `toml_edit`:
//! comments, blank lines, key spelling and table positions are all preserved,
//! and only the named keys are changed.

use std::collections::HashSet;
use std::path::Path;

use toml_edit::{DocumentMut, Item, Table, TableLike, Value};

use super::{decode_utf8, line_column, KeyPath, LivePatch, LiveWriteError};

/// Scope cleared by predicate: every key in the parent table that matches the
/// predicate.
#[derive(Clone)]
pub struct TomlClearScope {
    pub parent: KeyPath,
    pub is_floor: fn(&str) -> bool,
}

#[derive(Clone, Default)]
pub struct TomlPatch {
    /// Key fields cleared first. Keys also present in the target value are kept
    /// for `set` to update in place.
    pub clear: Vec<TomlClearScope>,
    /// Keys or whole tables removed by path (model names nested in user tables,
    /// the provider table written last time).
    pub remove: Vec<KeyPath>,
    /// Target values: replaced in place when present (keeping the original
    /// position and whitespace), appended when absent.
    pub set: Vec<(KeyPath, Item)>,
    /// Written only when absent.
    pub seed: Vec<(KeyPath, Item)>,
    /// Removed only when the current value equals one of these (whitespace and
    /// comments ignored). Skipped when the path appears in `set`.
    pub remove_if: Vec<(KeyPath, Vec<Value>)>,
}

impl TomlPatch {
    pub fn apply_to(&self, path: &Path, doc: &mut DocumentMut) -> Result<(), LiveWriteError> {
        let targets: HashSet<&KeyPath> = self.set.iter().map(|(key_path, _)| key_path).collect();
        let root = doc.as_table_mut();

        for scope in &self.clear {
            let Some(table) = table_at_mut(path, root, &scope.parent.0)? else {
                continue;
            };
            let doomed: Vec<String> = table
                .iter()
                .map(|(key, _)| key.to_string())
                .filter(|key| (scope.is_floor)(key) && !targets.contains(&scope.parent.child(key)))
                .collect();
            for key in doomed {
                table.remove(&key);
            }
        }

        for key_path in &self.remove {
            if targets.contains(key_path) {
                continue;
            }
            let (parent, key) = split(key_path);
            if let Some(table) = table_at_mut(path, root, parent)? {
                table.remove(key);
            }
        }

        for (key_path, item) in &self.set {
            let (parent, key) = split(key_path);
            let table = ensure_table_mut(path, root, parent)?;
            match table.get_mut(key) {
                Some(slot) => {
                    let mut replacement = item.clone();
                    keep_layout(slot, &mut replacement);
                    *slot = replacement;
                }
                None => {
                    table.insert(key, item.clone());
                }
            }
        }

        for (key_path, item) in &self.seed {
            let (parent, key) = split(key_path);
            let table = ensure_table_mut(path, root, parent)?;
            if !table.contains_key(key) {
                table.insert(key, item.clone());
            }
        }

        for (key_path, values) in &self.remove_if {
            if targets.contains(key_path) {
                continue;
            }
            let (parent, key) = split(key_path);
            let Some(table) = table_at_mut(path, root, parent)? else {
                continue;
            };
            let matches = table
                .get(key)
                .and_then(Item::as_value)
                .is_some_and(|current| values.iter().any(|value| same_value(current, value)));
            if matches {
                table.remove(key);
            }
        }

        Ok(())
    }
}

/// Modifies an already-parsed TOML document. A patch implementing this can be
/// handed straight to the engine: parse, modify, emit.
pub trait TomlDocPatch {
    fn apply_to(&self, path: &Path, doc: &mut DocumentMut) -> Result<(), LiveWriteError>;
}

impl<T: TomlDocPatch> LivePatch for T {
    fn apply(&self, path: &Path, pre: Option<&[u8]>) -> Result<Vec<u8>, LiveWriteError> {
        let mut doc = parse(path, pre)?;
        self.apply_to(path, &mut doc)?;
        Ok(doc.to_string().into_bytes())
    }
}

/// Applies several patches to the same document in order (for example editor
/// changes first, then swapping key fields).
pub struct TomlSteps<'a>(pub Vec<&'a dyn TomlDocPatch>);

impl TomlDocPatch for TomlSteps<'_> {
    fn apply_to(&self, path: &Path, doc: &mut DocumentMut) -> Result<(), LiveWriteError> {
        self.0.iter().try_for_each(|step| step.apply_to(path, doc))
    }
}

impl TomlDocPatch for TomlPatch {
    fn apply_to(&self, path: &Path, doc: &mut DocumentMut) -> Result<(), LiveWriteError> {
        Self::apply_to(self, path, doc)
    }
}

/// Parses the pre-write content; a missing file yields an empty document.
pub fn parse(path: &Path, pre: Option<&[u8]>) -> Result<DocumentMut, LiveWriteError> {
    let Some(bytes) = pre else {
        return Ok(DocumentMut::new());
    };
    let text = decode_utf8(path, bytes)?;
    text.parse::<DocumentMut>().map_err(|err| {
        let (line, column) = line_column(text, err.span().map_or(0, |span| span.start));
        LiveWriteError::Parse {
            path: path.to_path_buf(),
            line,
            column,
            message: err.message().to_string(),
        }
    })
}

/// The value spelling, without surrounding whitespace or trailing comments.
pub fn value_text(value: &Value) -> String {
    let mut value = value.clone();
    value.decor_mut().clear();
    value.to_string()
}

/// Values are the same when spelled the same, ignoring surrounding whitespace and
/// trailing comments.
pub fn same_value(left: &Value, right: &Value) -> bool {
    value_text(left) == value_text(right)
}

/// On in-place replacement, keeps the old item’s whitespace, comments and table
/// position; the line (or table) does not move after replacement.
fn keep_layout(old: &Item, new: &mut Item) {
    match (old, new) {
        (Item::Value(old), Item::Value(new)) => *new.decor_mut() = old.decor().clone(),
        (Item::Table(old), Item::Table(new)) => {
            *new.decor_mut() = old.decor().clone();
            if let Some(position) = old.position() {
                new.set_position(position);
            }
        }
        _ => {}
    }
}

fn split(key_path: &KeyPath) -> (&[String], &String) {
    key_path
        .split_last()
        .expect("patch paths must name a key, not the document root")
}

/// This path segment should be a table but is not.
pub fn shape_error(path: &Path, segments: &[String]) -> LiveWriteError {
    LiveWriteError::Shape {
        path: path.to_path_buf(),
        key_path: KeyPath(segments.to_vec()),
        expected: "a table",
    }
}

/// Finds a table (standard or inline) along the path; a missing segment returns
/// `None`, and a non-table segment is an error.
fn table_at_mut<'a>(
    path: &Path,
    root: &'a mut Table,
    segments: &[String],
) -> Result<Option<&'a mut dyn TableLike>, LiveWriteError> {
    let mut current: &mut dyn TableLike = root;
    for (depth, segment) in segments.iter().enumerate() {
        let Some(item) = current.get_mut(segment) else {
            return Ok(None);
        };
        current = item
            .as_table_like_mut()
            .ok_or_else(|| shape_error(path, &segments[..=depth]))?;
    }
    Ok(Some(current))
}

/// Finds a table along the path, creating missing levels as implicit standard
/// tables (no separate `[a]` header is emitted).
fn ensure_table_mut<'a>(
    path: &Path,
    root: &'a mut Table,
    segments: &[String],
) -> Result<&'a mut dyn TableLike, LiveWriteError> {
    let mut current: &mut dyn TableLike = root;
    for (depth, segment) in segments.iter().enumerate() {
        if !current.contains_key(segment) {
            let mut table = Table::new();
            table.set_implicit(true);
            current.insert(segment, Item::Table(table));
        }
        current = current
            .get_mut(segment)
            .and_then(Item::as_table_like_mut)
            .ok_or_else(|| shape_error(path, &segments[..=depth]))?;
    }
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_fields as floor;
    use toml_edit::value;

    fn apply(patch: &TomlPatch, pre: &str) -> String {
        let out = patch
            .apply(Path::new("config.toml"), Some(pre.as_bytes()))
            .expect("apply");
        String::from_utf8(out).expect("utf8")
    }

    fn table(text: &str) -> Item {
        let doc: DocumentMut = text.parse().expect("fragment");
        Item::Table(doc.as_table().clone())
    }

    const LIVE: &str = r#"# user comment
model_provider = "custom"
model = "gpt-a"   # trailing comment
approval_policy = "on-request"
openai_base_url = "https://stale.example/v1"

[model_providers.custom]
name = "A"
base_url = "https://a.example/v1"
experimental_bearer_token = "sk-a"

[agents]
default_subagent_model = "gpt-a-mini"
max_threads = 4

[mcp_servers.fs]
command = "fs-server"
"#;

    fn codex_patch() -> TomlPatch {
        TomlPatch {
            clear: vec![TomlClearScope {
                parent: KeyPath::root(),
                is_floor: |key| floor::CODEX_FLOOR_TOP.contains(&key),
            }],
            remove: floor::CODEX_FLOOR_NESTED
                .iter()
                .map(|segments| KeyPath::new(segments))
                .chain([KeyPath::new(floor::CODEX_PROVIDER_TABLE)])
                .collect(),
            set: vec![
                (KeyPath::new(&["model_provider"]), value("custom")),
                (KeyPath::new(&["model"]), value("gpt-b")),
                (
                    KeyPath::new(floor::CODEX_PROVIDER_TABLE),
                    table("name = \"B\"\nbase_url = \"https://b.example/v1\"\n"),
                ),
            ],
            ..TomlPatch::default()
        }
    }

    #[test]
    fn key_fields_are_replaced_and_everything_else_is_untouched() {
        let out = apply(&codex_patch(), LIVE);
        assert_eq!(
            out,
            r#"# user comment
model_provider = "custom"
model = "gpt-b"   # trailing comment
approval_policy = "on-request"

[model_providers.custom]
name = "B"
base_url = "https://b.example/v1"

[agents]
max_threads = 4

[mcp_servers.fs]
command = "fs-server"
"#
        );
        assert_eq!(
            apply(&codex_patch(), &out),
            out,
            "second write is byte-stable"
        );
    }

    #[test]
    fn a_new_table_goes_to_the_end() {
        let patch = TomlPatch {
            set: vec![(
                KeyPath::new(&["model", "grok-x"]),
                table("base_url = \"https://x.example\"\n"),
            )],
            ..TomlPatch::default()
        };
        let out = apply(
            &patch,
            "[models]\ndefault = \"grok-x\"\n\n[ui]\ntheme = \"dark\"\n",
        );
        assert_eq!(
            out,
            "[models]\ndefault = \"grok-x\"\n\n[ui]\ntheme = \"dark\"\n\n[model.grok-x]\nbase_url = \"https://x.example\"\n"
        );
    }

    #[test]
    fn remove_if_ignores_layout_and_yields_to_targets() {
        let patch = TomlPatch {
            remove_if: vec![(KeyPath::new(&["web_search"]), vec!["disabled".into()])],
            ..TomlPatch::default()
        };
        assert_eq!(
            apply(&patch, "web_search =   \"disabled\"  # x\nk = 1\n"),
            "k = 1\n"
        );
        assert_eq!(
            apply(&patch, "web_search = \"live\"\n"),
            "web_search = \"live\"\n"
        );
    }

    #[test]
    fn inline_tables_are_navigated_too() {
        let patch = TomlPatch {
            remove: vec![KeyPath::new(&["agents", "default_subagent_model"])],
            ..TomlPatch::default()
        };
        assert_eq!(
            apply(
                &patch,
                "agents = { default_subagent_model = \"m\", max_threads = 4 }\n"
            ),
            "agents = { max_threads = 4 }\n"
        );
    }

    #[test]
    fn broken_files_and_wrong_shapes_are_refused() {
        let err = TomlPatch::default()
            .apply(Path::new("c.toml"), Some(b"a = 1\nb = \n"))
            .expect_err("must refuse");
        match err {
            LiveWriteError::Parse { line, .. } => assert_eq!(line, 2),
            other => panic!("unexpected error: {other:?}"),
        }

        let patch = TomlPatch {
            set: vec![(KeyPath::new(&["agents", "x"]), value(1))],
            ..TomlPatch::default()
        };
        let err = patch
            .apply(Path::new("c.toml"), Some(b"agents = \"oops\"\n"))
            .expect_err("must refuse");
        assert!(matches!(err, LiveWriteError::Shape { .. }), "{err:?}");
    }
}
