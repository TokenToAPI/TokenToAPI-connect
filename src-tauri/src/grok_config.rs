//! Official Grok Build custom-model configuration. Never changes shell or OAuth credentials.
use crate::{
    models::ValidatedInput,
    patch::{toml::TomlPatch, KeyPath, LivePatch},
    storage::{self, Change, FileKind, Paths},
};
use sha2::{Digest, Sha256};
use toml_edit::{value, Array, ArrayOfTables, Item, Table};

const OWNER: &str = "Managed by TokenToAPI Connect";
fn key(id: &str) -> String {
    let readable: String = id
        .chars()
        .take(80)
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let hash = format!("{:x}", Sha256::digest(id.as_bytes()));
    format!("tokentoapi-{readable}-{}", &hash[..12])
}

pub fn change(paths: &Paths, input: &ValidatedInput) -> Result<Change, String> {
    let kind = FileKind::GrokConfig;
    let path = paths.target(kind);
    let before = storage::read_optional(&path)?;
    let parsed = crate::patch::toml::parse(&path, before.as_deref().map(str::as_bytes))
        .map_err(|e| e.to_string())?;
    let catalog: serde_json::Value = serde_json::from_str(
        input
            .catalog
            .as_deref()
            .ok_or("Fetch model capabilities before importing Grok CLI.")?,
    )
    .map_err(|_| "Invalid Grok model capabilities.")?;
    let rows = catalog["models"]
        .as_array()
        .ok_or("Invalid Grok model capabilities.")?;
    let mut patch = TomlPatch::default();
    if let Some(models) = parsed.get("model").and_then(Item::as_table_like) {
        for (id, entry) in models.iter() {
            if id.starts_with("tokentoapi-")
                && entry.get("description").and_then(Item::as_str) == Some(OWNER)
            {
                patch.remove.push(KeyPath::new(&["model", id]));
            }
        }
    }
    let mut allowed = Array::new();
    for id in &input.selected_models {
        let alias = key(id);
        if let Some(existing) = parsed.get("model").and_then(|models| models.get(&alias)) {
            if existing.get("description").and_then(Item::as_str) != Some(OWNER) {
                return Err("A Grok model uses a reserved Connect name. Rename that model before importing; no configuration was changed.".into());
            }
        }
        let row = rows
            .iter()
            .find(|row| row["slug"].as_str() == Some(id))
            .ok_or("A selected Grok model has no capabilities.")?;
        crate::codex_context::validate(row, true)?;
        let context = row["context_window"]
            .as_i64()
            .or(row["max_context_window"].as_i64())
            .ok_or("Missing Grok context capacity.")?;
        let mut model = Table::new();
        for (field, content) in [
            ("model", id.as_str()),
            ("name", id.as_str()),
            ("description", OWNER),
            ("api_key", input.api_key.as_str()),
            ("api_backend", "responses"),
            ("reasoning_summary", "none"),
        ] {
            model.insert(field, value(content));
        }
        model.insert("base_url", value(format!("{}/v1", input.root)));
        model.insert("context_window", value(context));
        model.insert("supports_backend_search", value(false));
        let mut efforts = ArrayOfTables::new();
        if let Some(levels) = row["supported_reasoning_levels"].as_array() {
            for level in levels {
                let Some(effort) = level["effort"].as_str() else {
                    continue;
                };
                if !["none", "minimal", "low", "medium", "high", "xhigh"].contains(&effort) {
                    continue;
                }
                let mut option = Table::new();
                for field in ["id", "value", "label"] {
                    option.insert(field, value(effort));
                }
                option.insert(
                    "default",
                    value(row["default_reasoning_level"].as_str() == Some(effort)),
                );
                efforts.push(option);
            }
        }
        model.insert("supports_reasoning_effort", value(!efforts.is_empty()));
        model.insert("reasoning_efforts", Item::ArrayOfTables(efforts));
        patch
            .set
            .push((KeyPath::new(&["model", &alias]), Item::Table(model)));
        allowed.push(alias);
    }
    for field in [
        "default",
        "session_summary",
        "prompt_suggestion",
        "web_search",
        "image_description",
    ] {
        patch
            .set
            .push((KeyPath::new(&["models", field]), value(key(&input.model))));
    }
    patch
        .set
        .push((KeyPath::new(&["models", "allowed_models"]), value(allowed)));
    // Disable first-party model/asset discovery; selected entries are complete and refreshed in Connect.
    patch
        .set
        .push((KeyPath::new(&["features", "remote_fetch"]), value(false)));
    let after = String::from_utf8(
        patch
            .apply(&path, before.as_deref().map(str::as_bytes))
            .map_err(|e| e.to_string())?,
    )
    .map_err(|_| "Configuration encoding error.")?;
    Ok(Change {
        kind,
        before,
        after,
    })
}
