use crate::{
    models::{Client, ValidatedInput},
    patch::{
        json::{ClearScope, JsonPatch},
        toml::TomlPatch,
        KeyPath, LivePatch,
    },
    provider_fields as floor,
    storage::{self, Change, FileKind, Paths},
};
use serde::Serialize;
use serde_json::json;
use toml_edit::{value, Item, Table};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub provider_name: Option<String>,
    pub client: Client,
    pub model: String,
    pub selected_models: Vec<String>,
    pub base_url: String,
    pub changed: bool,
    pub config_path: String,
    pub next_steps: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn apply(paths: &Paths, input: ValidatedInput) -> Result<ImportResult, String> {
    input.options.validate(&input)?;
    let resolved = paths.for_client(input.client)?;
    let paths = &resolved;
    let _lock = paths.lock()?;
    let mut compatibility = crate::compatibility::inspect(paths, input.client);
    if let Some(blocked) = compatibility.blocked {
        return Err(blocked);
    }
    let mut changes = Vec::new();
    match input.client {
        Client::Grok => changes.push(crate::grok_config::change(paths, &input)?),
        Client::ClaudeDesktop => changes.extend(crate::desktop_config::changes(paths, &input)?),
        Client::Claude | Client::ClaudeVscode => {
            let kind = FileKind::ClaudeSettings;
            let path = paths.target(kind);
            let before = storage::read_optional(&path)?;
            let mut patch = JsonPatch {
                clear: vec![
                    ClearScope {
                        parent: KeyPath::root(),
                        is_floor: floor::claude_floor_top,
                    },
                    ClearScope {
                        parent: KeyPath::new(&["env"]),
                        is_floor: if input.options.artifact_mode
                            == crate::setup_options::ArtifactMode::Preserve
                        {
                            floor::claude_floor_env_preserving_artifact
                        } else {
                            floor::claude_floor_env
                        },
                    },
                ],
                ..JsonPatch::default()
            };
            // Only provider/auth/model settings change; hooks, permissions and MCP remain untouched.
            patch
                .set
                .push((KeyPath::new(&["model"]), json!(input.model)));
            let options: Vec<_> = input
                .selected_models
                .iter()
                .map(|id| json!({"model": id, "label": id, "description": "Called through the selected gateway"}))
                .collect();
            patch.set.push((
                KeyPath::new(&["modelPicker"]),
                json!({"replaceBuiltInOptions": true, "options": options}),
            ));
            patch.set.push((
                KeyPath::new(&["availableModels"]),
                json!(input.selected_models),
            ));
            for (key, val) in crate::claude_config::environment(&input) {
                patch
                    .set
                    .push((KeyPath::new(&["env", key.as_str()]), json!(val)));
            }
            let after = String::from_utf8(
                patch
                    .apply(&path, before.as_deref().map(str::as_bytes))
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|_| "Configuration encoding error.")?;
            changes.push(Change {
                kind,
                before,
                after,
            });
            if input.client == Client::ClaudeVscode {
                changes.push(crate::vscode_config::change(paths, &input)?);
            }
        }
        Client::Codex | Client::CodexDesktop => {
            let kind = FileKind::CodexConfig;
            let path = paths.target(kind);
            let before = storage::read_optional(&path)?;
            let parsed = crate::patch::toml::parse(&path, before.as_deref().map(str::as_bytes))
                .map_err(|e| e.to_string())?;
            if parsed
                .get("profile")
                .and_then(Item::as_str)
                .is_some_and(|s| !s.trim().is_empty())
            {
                return Err(
                    "Codex currently uses a custom profile; switch back to the default configuration before importing to avoid overwriting it."
                        .into(),
                );
            }
            let mut provider = Table::new();
            provider.insert("name", value("TokenToAPI Connect"));
            provider.insert("base_url", value(format!("{}/v1", input.root)));
            provider.insert("wire_api", value("responses"));
            provider.insert("experimental_bearer_token", value(&input.api_key));
            provider.insert("requires_openai_auth", value(false));
            provider.insert("supports_websockets", value(false));
            let mut patch = TomlPatch::default();
            for id in ["openai", "ollama", "lmstudio"] {
                patch.remove.push(KeyPath::new(&["model_providers", id]));
            }
            for segments in floor::CODEX_FLOOR_NESTED {
                patch.remove.push(KeyPath::new(segments));
            }
            for key in [
                "model_reasoning_effort",
                "plan_mode_reasoning_effort",
                "review_model",
                "model_catalog_json",
                "openai_base_url",
                "experimental_bearer_token",
                "base_url",
                "wire_api",
            ] {
                patch.remove.push(KeyPath::new(&[key]));
            }
            for (key, val) in [
                ("model", input.model.as_str()),
                ("model_provider", "tokentoapi-connect"),
                ("web_search", input.options.web_search_value(&parsed)?),
            ] {
                patch.set.push((KeyPath::new(&[key]), value(val)));
            }
            patch.set.push((
                KeyPath::new(&["model_providers", "tokentoapi-connect"]),
                Item::Table(provider),
            ));
            if let Some(catalog) = &input.catalog {
                let catalog_path = paths.target(FileKind::CodexCatalog);
                let catalog_before = storage::read_optional(&catalog_path)?;
                let same_route = parsed.get("model_provider").and_then(Item::as_str)
                    == Some("tokentoapi-connect")
                    && parsed.get("model_catalog_json").and_then(Item::as_str)
                        == Some(catalog_path.to_string_lossy().as_ref())
                    && parsed
                        .get("model_providers")
                        .and_then(|providers| providers.get("tokentoapi-connect"))
                        .and_then(|provider| provider.get("base_url"))
                        .and_then(Item::as_str)
                        == Some(format!("{}/v1", input.root).as_str());
                let cached = storage::read_optional(&paths.codex.join("models_cache.json"))
                    .ok()
                    .flatten();
                let catalog = crate::codex_context::prepare_import(
                    catalog,
                    if same_route {
                        catalog_before.as_deref()
                    } else {
                        None
                    },
                    cached.as_deref(),
                )?;
                let catalog_json: serde_json::Value =
                    serde_json::from_str(&catalog).map_err(|_| {
                        "The generated model catalog is invalid; no configuration was written."
                    })?;
                if let Some(sources) =
                    catalog_json["_tokentoapi_connect"]["input_modality_sources"].as_object()
                {
                    let unknown: Vec<_> = sources
                        .iter()
                        .filter(|(_, source)| *source == "unknown" || *source == "gateway-invalid")
                        .map(|(id, _)| id.as_str())
                        .collect();
                    if !unknown.is_empty() {
                        compatibility.warnings.push(format!("Image support could not be verified for: {}. These models use text-only input until the gateway supplies input capabilities. Fetch models again after the gateway metadata is corrected, then import again.", unknown.join(", ")));
                    }
                }
                if let Some(levels) = catalog_json["models"]
                    .as_array()
                    .and_then(|models| models.iter().find(|model| model["slug"] == input.model))
                    .and_then(|model| model["supported_reasoning_levels"].as_array())
                {
                    for key in ["model_reasoning_effort", "plan_mode_reasoning_effort"] {
                        if let Some(previous) = parsed.get(key) {
                            let effort = previous
                                .as_str()
                                .map(str::to_owned)
                                .or_else(|| previous.as_integer().map(|number| number.to_string()));
                            if effort.is_some_and(|effort| {
                                levels
                                    .iter()
                                    .any(|level| level["effort"].as_str() == Some(effort.as_str()))
                            }) {
                                let preserved = previous
                                    .as_integer()
                                    .map(|number| value(number.to_string()))
                                    .unwrap_or_else(|| previous.clone());
                                patch.set.push((KeyPath::new(&[key]), preserved));
                            }
                        }
                    }
                }
                patch.set.push((
                    KeyPath::new(&["model_catalog_json"]),
                    value(catalog_path.to_string_lossy().as_ref()),
                ));
                changes.push(Change {
                    kind: FileKind::CodexCatalog,
                    before: catalog_before,
                    after: catalog,
                });
            }
            let after = String::from_utf8(
                patch
                    .apply(&path, before.as_deref().map(str::as_bytes))
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|_| "Configuration encoding error.")?;
            changes.push(Change {
                kind,
                before,
                after,
            });
        }
    }
    let changed = storage::commit(paths, input.client, changes)?;
    Ok(ImportResult {
        provider_name: None,
        client: input.client,
        model: input.model,
        selected_models: input.selected_models,
        base_url: input.root,
        changed,
        config_path: paths
            .target(FileKind::primary(input.client))
            .to_string_lossy()
            .into_owned(),
        next_steps: crate::compatibility::next_steps(input.client),
        warnings: compatibility.warnings,
    })
}
