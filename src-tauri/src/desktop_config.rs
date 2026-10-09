// Claude Desktop activation layout adapted from CC Switch (MIT),
// claude_desktop_config.rs. See UPSTREAM.md for provenance.
use crate::{
    models::ValidatedInput,
    patch::{json::JsonPatch, KeyPath, LivePatch},
    storage::{self, Change, FileKind, Paths},
};
use serde_json::{json, Value};

// A separate profile keeps the user's other gateways and CC Switch profiles intact.
pub const PROFILE_ID: &str = "761c488f-5a98-4c98-bac9-8b667c5be40a";

fn object(text: Option<&str>) -> Result<Value, String> {
    let value: Value = serde_json::from_str(text.unwrap_or("{}")).map_err(|_| {
        "The Claude Desktop configuration is not valid JSON; the configuration was not modified."
    })?;
    if !value.is_object() {
        return Err("The Claude Desktop configuration has an invalid format; the configuration was not modified.".into());
    }
    Ok(value)
}

fn patch(paths: &Paths, kind: FileKind, fields: Value) -> Result<Change, String> {
    let path = paths.target(kind);
    let before = storage::read_optional(&path)?;
    let set = fields
        .as_object()
        .ok_or("Invalid desktop configuration fields.")?
        .iter()
        .map(|(key, value)| (KeyPath::new(&[key]), value.clone()))
        .collect();
    let bytes = JsonPatch {
        set,
        ..JsonPatch::default()
    }
    .apply(&path, before.as_deref().map(str::as_bytes))
    .map_err(|e| e.to_string())?;
    Ok(Change {
        kind,
        before,
        after: String::from_utf8(bytes).map_err(|_| "Configuration encoding error.")?,
    })
}

pub fn changes(paths: &Paths, input: &ValidatedInput) -> Result<Vec<Change>, String> {
    // The desktop menu validates Claude family IDs. Do not silently substitute a
    // different model or pretend arbitrary OpenAI aliases are Claude models.
    if input
        .selected_models
        .iter()
        .any(|id| !supported_model_id(id))
    {
        return Err("Claude Desktop requires claude-sonnet-*, claude-opus-*, claude-haiku-* or claude-fable-* model IDs. Select only compatible models; other aliases require the gateway to provide Claude routing.".into());
    }
    let mut ordered = vec![input.model.clone()];
    ordered.extend(
        input
            .selected_models
            .iter()
            .filter(|id| **id != input.model)
            .cloned(),
    );
    let profile = patch(
        paths,
        FileKind::ClaudeDesktopProfile,
        json!({
            "inferenceProvider": "gateway",
            "inferenceGatewayBaseUrl": input.root,
            "inferenceGatewayApiKey": input.api_key,
            "inferenceGatewayAuthScheme": "bearer",
            "inferenceCredentialKind": "static",
            "modelDiscoveryEnabled": false,
            "inferenceModels": ordered,
        }),
    )?;
    let before = storage::read_optional(&paths.target(FileKind::ClaudeDesktopMeta))?;
    let mut meta = object(before.as_deref())?;
    let entries = meta
        .as_object_mut()
        .unwrap()
        .entry("entries")
        .or_insert(json!([]))
        .as_array_mut()
        .ok_or("The entries format in the Claude Desktop configuration registry is invalid; the configuration was not modified.")?;
    if entries
        .iter()
        .any(|v| !v.is_object() || v.get("id").and_then(Value::as_str).is_none())
    {
        return Err("The Claude Desktop configuration registry contains unrecognizable entries; the configuration was not modified.".into());
    }
    if let Some(entry) = entries.iter_mut().find(|v| v["id"] == PROFILE_ID) {
        entry["name"] = json!("TokenToAPI Connect");
    } else {
        entries.push(json!({ "id": PROFILE_ID, "name": "TokenToAPI Connect" }));
    }
    meta["appliedId"] = json!(PROFILE_ID);
    // Activate only after the credential, catalog and registry are ready.
    Ok(vec![
        profile,
        Change {
            kind: FileKind::ClaudeDesktopMeta,
            before,
            after: serde_json::to_string_pretty(&meta)
                .map_err(|_| "Could not generate the desktop configuration registry.")?,
        },
        patch(
            paths,
            FileKind::ClaudeThreepConfig,
            json!({"deploymentMode":"3p"}),
        )?,
        patch(
            paths,
            FileKind::ClaudeDesktopConfig,
            json!({"deploymentMode":"3p"}),
        )?,
    ])
}

pub fn supported_model_id(id: &str) -> bool {
    let normalized = id.to_ascii_lowercase();
    let id = normalized.strip_prefix("anthropic/").unwrap_or(&normalized);
    [
        "claude-sonnet-",
        "claude-opus-",
        "claude-haiku-",
        "claude-fable-",
    ]
    .iter()
    .any(|prefix| {
        id.strip_prefix(prefix)
            .is_some_and(|tail| !tail.is_empty() && !tail.contains('['))
    })
}
