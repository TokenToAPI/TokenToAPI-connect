//! Gemini CLI user configuration; never edits shell profiles or OAuth storage.
use crate::{
    models::ValidatedInput,
    patch::{json::JsonPatch, KeyPath, LivePatch},
    storage::{self, Change, FileKind, Paths},
};
use serde_json::json;

fn quoted(value: &str) -> Result<String, String> {
    // dotenv performs no interpolation. Choose a literal delimiter, without
    // changing key bytes or interpreting backslash-n sequences.
    for quote in ['\'', '`'] {
        if !value.contains(quote) && !value.chars().any(char::is_control) {
            return Ok(format!("{quote}{value}{quote}"));
        }
    }
    Err(
        "The Gemini API value contains unsupported quote characters; no configuration was changed."
            .into(),
    )
}

fn environment(before: &str, input: &ValidatedInput) -> Result<String, String> {
    let managed = [
        ("GEMINI_API_KEY", input.api_key.as_str()),
        ("GOOGLE_GEMINI_BASE_URL", input.root.as_str()),
        ("GEMINI_MODEL", input.model.as_str()),
        ("GOOGLE_GENAI_API_VERSION", "v1beta"),
        ("GEMINI_API_KEY_AUTH_MECHANISM", "x-goog-api-key"),
    ];
    let mut after = String::new();
    for line in before.lines() {
        let text = line.trim();
        if text.is_empty() || text.starts_with('#') {
            after.push_str(line);
            after.push('\n');
            continue;
        }
        let assignment = text.strip_prefix("export ").unwrap_or(text).trim_start();
        let Some((name, value)) = assignment.split_once('=') else {
            return Err(
                "Gemini's environment file has an unsupported entry; no configuration was changed."
                    .into(),
            );
        };
        let value = value.trim();
        // Refuse multiline quoting rather than risk treating its continuation
        // as another setting. Original bytes remain available for exact restore.
        if let Some(delimiter) = value
            .chars()
            .next()
            .filter(|c| ['\'', '"', '`'].contains(c))
        {
            if !value[1..].contains(delimiter) {
                return Err("Gemini's environment file uses multiline values; no configuration was changed.".into());
            }
        }
        if !managed.iter().any(|(key, _)| name.trim() == *key) {
            after.push_str(line);
            after.push('\n');
        }
    }
    for (name, value) in managed {
        after.push_str(&format!("{name}={}\n", quoted(value)?));
    }
    Ok(after)
}

pub fn changes(paths: &Paths, input: &ValidatedInput) -> Result<Vec<Change>, String> {
    let kind = FileKind::GeminiSettings;
    let path = paths.target(kind);
    let before = storage::read_optional(&path)?;
    let patch = JsonPatch {
        set: vec![
            (
                KeyPath::new(&["security", "auth", "selectedType"]),
                json!("gemini-api-key"),
            ),
            (KeyPath::new(&["model", "name"]), json!(input.model)),
        ],
        ..Default::default()
    };
    let after = String::from_utf8(
        patch
            .apply(&path, before.as_deref().map(str::as_bytes))
            .map_err(|_| "Gemini settings are invalid; no configuration was changed.")?,
    )
    .map_err(|_| "Gemini settings encoding is invalid.")?;
    let env_kind = FileKind::GeminiEnv;
    let env_before = storage::read_optional(&paths.target(env_kind))?;
    let env_after = environment(env_before.as_deref().unwrap_or(""), input)?;
    Ok(vec![
        Change {
            kind,
            before,
            after,
        },
        Change {
            kind: env_kind,
            before: env_before,
            after: env_after,
        },
    ])
}
