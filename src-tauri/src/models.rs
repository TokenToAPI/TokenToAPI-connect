use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Client {
    Claude,
    Codex,
    #[serde(rename = "claude-desktop")]
    ClaudeDesktop,
    #[serde(rename = "claude-vscode")]
    ClaudeVscode,
    #[serde(rename = "codex-desktop")]
    CodexDesktop,
}
impl Client {
    pub const ALL: [Self; 5] = [
        Self::CodexDesktop,
        Self::Codex,
        Self::ClaudeDesktop,
        Self::Claude,
        Self::ClaudeVscode,
    ];
    pub fn is_codex(self) -> bool {
        matches!(self, Self::Codex | Self::CodexDesktop)
    }
    // Both Codex entry points share one configuration and one restore point.
    pub fn backup_client(self) -> Self {
        if self == Self::CodexDesktop {
            Self::Codex
        } else {
            self
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::ClaudeDesktop => "claude-desktop",
            Self::ClaudeVscode => "claude-vscode",
            Self::CodexDesktop => "codex-desktop",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Claude => "Claude Code Terminal",
            Self::Codex => "Codex Terminal",
            Self::ClaudeDesktop => "Claude Desktop",
            Self::ClaudeVscode => "Claude Code · VS Code",
            Self::CodexDesktop => "Codex Desktop",
        }
    }
}

// Values containing credentials deliberately do not implement Debug or Serialize.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiscoveryInput {
    pub client: Client,
    pub api_key: String,
    pub base_url: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectionInput {
    pub discovery_id: String,
    pub selected_models: Vec<String>,
    pub default_model: String,
    #[serde(default)]
    pub options: crate::setup_options::SetupOptions,
}
#[derive(Clone, Debug, Serialize)]
pub struct ModelChoice {
    pub id: String,
    pub name: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryResponse {
    pub discovery_id: String,
    pub client: Client,
    pub base_url: String,
    pub models: Vec<ModelChoice>,
}

pub struct DiscoveredModels {
    pub client: Client,
    pub api_key: String,
    pub root: String,
    pub payload: Value,
    pub choices: Vec<ModelChoice>,
}
pub struct ValidatedInput {
    pub client: Client,
    pub api_key: String,
    pub root: String,
    pub model: String,
    pub selected_models: Vec<String>,
    pub catalog: Option<String>,
    pub options: crate::setup_options::SetupOptions,
}
pub struct DiscoverySession {
    id: String,
    created: Instant,
    data: DiscoveredModels,
}
impl DiscoverySession {
    pub fn new(data: DiscoveredModels) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Self {
            id: format!("{now:x}-{:x}", NEXT.fetch_add(1, Ordering::Relaxed)),
            created: Instant::now(),
            data,
        }
    }
    pub fn response(&self) -> DiscoveryResponse {
        DiscoveryResponse {
            discovery_id: self.id.clone(),
            client: self.data.client,
            base_url: self.data.root.clone(),
            models: self.data.choices.clone(),
        }
    }
    pub fn prepare(&self, input: &SelectionInput) -> Result<ValidatedInput, String> {
        if input.discovery_id != self.id || self.created.elapsed() > Duration::from_secs(15 * 60) {
            return Err(
                "The model list has expired; fetch the models again before importing.".into(),
            );
        }
        let mut selected =
            prepare_selection(&self.data, &input.selected_models, &input.default_model)?;
        input.options.validate(&selected)?;
        selected.options = input.options.clone();
        Ok(selected)
    }
}

pub fn normalize_root(input: &str) -> Result<String, String> {
    let mut url = reqwest::Url::parse(input.trim())
        .map_err(|_| "The API address format is invalid; enter a complete https:// address.")?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if url.scheme() != "https" && !(url.scheme() == "http" && local) {
        return Err("The API address must use HTTPS; local debug addresses may use HTTP.".into());
    }
    if url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(
            "The API address must not contain a username, password, query parameters or an anchor."
                .into(),
        );
    }
    let path = url.path().trim_end_matches('/').to_string();
    if path.ends_with("/responses")
        || path.ends_with("/messages")
        || path.ends_with("/chat/completions")
    {
        return Err("Enter the API base address; do not include endpoint paths such as /responses or /messages.".into());
    }
    url.set_path(path.strip_suffix("/v1").unwrap_or(&path));
    Ok(url.as_str().trim_end_matches('/').to_string())
}

pub fn catalog_models(payload: &Value) -> Vec<String> {
    let rows = model_rows(payload);
    let mut models = Vec::new();
    if let Some(rows) = rows {
        for row in rows.iter().take(5000) {
            if let Some(id) = model_identifier(row) {
                let id = id.trim();
                if is_chat_model(id)
                    && !id.is_empty()
                    && id.len() <= 200
                    && !id.chars().any(char::is_control)
                    && !models.iter().any(|s| s == id)
                {
                    models.push(id.to_string());
                }
            }
        }
    }
    models
}

pub fn is_chat_model(id: &str) -> bool {
    let id = id.to_ascii_lowercase();
    ![
        "gpt-image",
        "dall-e",
        "whisper-",
        "tts-",
        "text-embedding-",
        "sora-",
    ]
    .iter()
    .any(|prefix| id == *prefix || id.starts_with(prefix))
}

fn model_rows(payload: &Value) -> Option<&Vec<Value>> {
    payload
        .get("data")
        .and_then(Value::as_array)
        .or_else(|| payload.get("models").and_then(Value::as_array))
}

fn model_identifier(row: &Value) -> Option<&str> {
    row.get("id")
        .and_then(Value::as_str)
        .or_else(|| row.get("slug").and_then(Value::as_str))
}

const CODEX_MODEL_METADATA_FIELDS: &[&str] = &[
    "display_name",
    "description",
    "context_window",
    "max_context_window",
    "auto_compact_token_limit",
    "effective_context_window_percent",
    "input_modalities",
    "default_reasoning_level",
    "supported_reasoning_levels",
    "supports_reasoning_summaries",
    "multi_agent_reasoning_effort",
];

fn reasoning_description(effort: &str) -> &'static str {
    match effort {
        "none" => "Disable reasoning",
        "minimal" => "Fastest responses with minimal reasoning",
        "low" => "Fast responses with lighter reasoning",
        "medium" => "Balances speed and reasoning depth for everyday tasks",
        "high" => "Greater reasoning depth for complex problems",
        "xhigh" => "Extra high reasoning depth for complex problems",
        _ => "",
    }
}

fn effort_string(value: &mut Value) -> Result<(), String> {
    if value.is_number() {
        *value = json!(value.to_string());
    }
    if !value
        .as_str()
        .is_some_and(|effort| !effort.trim().is_empty() && !effort.chars().any(char::is_control))
    {
        return Err(
            "The gateway declared an invalid reasoning effort; no configuration was written."
                .into(),
        );
    }
    Ok(())
}

fn normalize_reasoning_levels(value: &mut Value) -> Result<(), String> {
    let values = value
        .as_array()
        .cloned()
        .ok_or("The gateway reasoning levels must be an array; no configuration was written.")?;
    let mut levels = Vec::new();
    for value in values {
        let (mut effort, description) = match value {
            Value::String(effort) => (json!(effort), None),
            Value::Object(object) => (
                object
                    .get("effort")
                    .cloned()
                    .ok_or("A gateway reasoning level is missing its effort; no configuration was written.")?,
                object.get("description").cloned(),
            ),
            _ => {
                return Err(
                    "The gateway declared an invalid reasoning level; no configuration was written."
                        .into(),
                )
            }
        };
        effort_string(&mut effort)?;
        let label = effort.as_str().unwrap();
        let description =
            match description {
                Some(Value::String(text)) if !text.trim().is_empty() => text,
                Some(Value::String(_)) | None | Some(Value::Null) => {
                    let known = reasoning_description(label);
                    if known.is_empty() {
                        format!("{label} reasoning effort")
                    } else {
                        known.to_string()
                    }
                }
                _ => return Err(
                    "A gateway reasoning description must be text; no configuration was written."
                        .into(),
                ),
            };
        if !levels.iter().any(|level: &Value| level["effort"] == effort) {
            levels.push(json!({"effort":effort,"description":description}));
        }
    }
    *value = Value::Array(levels);
    Ok(())
}

fn model_profile<'a>(profiles: &'a Value, id: &str) -> Option<&'a Value> {
    let profiles = profiles.as_object()?;
    profiles.get(id).or_else(|| {
        profiles
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(id))
            .map(|(_, row)| row)
    })
}

fn find_payload_model<'a>(payload: &'a Value, id: &str) -> Option<&'a Value> {
    ["data", "models"].into_iter().find_map(|key| {
        payload.get(key).and_then(Value::as_array).and_then(|rows| {
            rows.iter()
                .find(|row| model_identifier(row).is_some_and(|candidate| candidate.trim() == id))
        })
    })
}

fn normalize_catalog_shape(row: &mut Value) {
    if row["effective_context_window_percent"].is_null() {
        row["effective_context_window_percent"] = json!(95);
    }
    if !row.get("input_modalities").is_some_and(|value| {
        value.as_array().is_some_and(|items| {
            !items.is_empty() && items.iter().all(|item| item.as_str().is_some())
        })
    }) {
        row["input_modalities"] = json!(["text"]);
    }
}

pub async fn discover(input: DiscoveryInput) -> Result<DiscoveredModels, String> {
    let api_key = input.api_key.trim().to_string();
    if api_key.is_empty()
        || api_key.len() > 4096
        || api_key.chars().any(char::is_whitespace)
        || api_key.chars().any(char::is_control)
    {
        return Err("Paste the full API key; it must not contain spaces or newlines.".into());
    }
    let root = normalize_root(&input.base_url)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Could not initialize the network connection.")?;
    const CODEX_CATALOG_CLIENT_VERSION: &str = "0.162.0";
    let endpoint = format!("{root}/v1/models");
    let fetch = |codex_manifest: bool| {
        let mut request = client
            .get(&endpoint)
            .bearer_auth(&api_key)
            .header("Accept", "application/json");
        if codex_manifest {
            request = request.query(&[("client_version", CODEX_CATALOG_CLIENT_VERSION)]);
        }
        async move {
            request.send().await.map_err(|error| {
                if error.is_timeout() {
                    "Connection timed out; check your network and the API address."
                } else {
                    "Could not reach the API; check your network, certificates and the API address."
                }
                .to_string()
            })
        }
    };
    // Codex's manifest may include image and reasoning capabilities omitted from the plain list.
    // Credentials remain on this origin, with one narrow fallback for gateways that reject the query.
    let codex_client = input.client.is_codex();
    let mut response = fetch(codex_client).await?;
    if codex_client && matches!(response.status().as_u16(), 400 | 404 | 405 | 422) {
        response = fetch(false).await?;
    }
    if !response.status().is_success() {
        return Err(match response.status().as_u16() {
            401 => "The API key failed authentication; check that the full key was copied.".into(),
            403 => "This API key lacks access; check the group, validity period or account status.".into(),
            404 | 405 => {
                "This address did not provide a model catalog; check the API base address. The first version requires the /v1/models endpoint.".into()
            }
            429 => "Too many requests or rate limit reached; please retry later.".into(),
            code => format!("The model catalog is temporarily unavailable (HTTP {code}); the configuration was not written."),
        });
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Failed to read the model catalog; please retry.")?
    {
        if bytes.len() + chunk.len() > 2 * 1024 * 1024 {
            return Err(
                "The model catalog is too large; the configuration was not written.".into(),
            );
        }
        bytes.extend_from_slice(&chunk);
    }
    let payload: Value = serde_json::from_slice(&bytes)
        .map_err(|_| "The model catalog returned by the API is not valid JSON.")?;
    let models = catalog_models(&payload);
    if models.is_empty() {
        return Err("The API key returned an empty model catalog; check whether the group has available models.".into());
    }
    let rows = payload
        .get("data")
        .or_else(|| payload.get("models"))
        .and_then(Value::as_array);
    let choices = models
        .into_iter()
        .map(|id| {
            let label = rows
                .and_then(|rows| {
                    rows.iter()
                        .find(|row| model_identifier(row).is_some_and(|v| v.trim() == id))
                })
                .and_then(|row| row.get("display_name").or_else(|| row.get("name")))
                .and_then(Value::as_str)
                .filter(|name| {
                    !name.is_empty() && name.len() <= 200 && !name.chars().any(char::is_control)
                })
                .unwrap_or(&id)
                .to_string();
            ModelChoice { id, name: label }
        })
        .collect();
    Ok(DiscoveredModels {
        client: input.client,
        api_key,
        root,
        payload,
        choices,
    })
}

pub fn prepare_selection(
    data: &DiscoveredModels,
    selected: &[String],
    default_model: &str,
) -> Result<ValidatedInput, String> {
    if selected.is_empty() || selected.len() > 200 {
        return Err("Select 1–200 models to import.".into());
    }
    let available: HashSet<&str> = data.choices.iter().map(|m| m.id.as_str()).collect();
    let mut seen = HashSet::new();
    for id in selected {
        if !available.contains(id.as_str()) || !is_chat_model(id) || !seen.insert(id) {
            return Err(
                "A selected model is not in the catalog returned for this key, or it was selected more than once. Fetch the models again.".into(),
            );
        }
    }
    if !selected.iter().any(|id| id == default_model) {
        return Err("Specify a default model from the selected ones.".into());
    }
    let catalog = if data.client.is_codex() {
        Some(build_codex_catalog(&data.payload, selected)?)
    } else {
        None
    };
    Ok(ValidatedInput {
        client: data.client,
        api_key: data.api_key.clone(),
        root: data.root.clone(),
        model: default_model.into(),
        selected_models: selected.to_vec(),
        catalog,
        options: crate::setup_options::SetupOptions::default(),
    })
}

// CC Switch's native Responses catalog template; gateway-supplied metadata takes precedence.
// The selected list is the complete external catalog, so unchecked models never leak into it.
pub fn build_codex_catalog(payload: &Value, selected: &[String]) -> Result<String, String> {
    let template: Value = serde_json::from_str(include_str!(
        "resources/codex_native_responses_template.json"
    ))
    .map_err(|_| "The model catalog template is unavailable.")?;
    let profiles: Value = serde_json::from_str(include_str!("resources/codex_model_profiles.json"))
        .map_err(|_| "The built-in Codex model profiles are unavailable.")?;
    let mut context_sources = json!({});
    let mut gateway_context_fields = json!({});
    let entries: Vec<Value> = selected
        .iter()
        .enumerate()
        .map(|(index, id)| -> Result<Value, String> {
            let mut row = template.clone();
            row["context_window"] = Value::Null;
            row["max_context_window"] = Value::Null;
            row["auto_compact_token_limit"] = Value::Null;
            row["effective_context_window_percent"] = json!(95);
            row["input_modalities"] = json!(["text"]);
            row["default_reasoning_level"] = Value::Null;
            row["supported_reasoning_levels"] = json!([]);
            row["supports_reasoning_summaries"] = json!(false);
            row["display_name"] = json!(id);
            row["description"] = json!(id);
            let profile = model_profile(&profiles, id);
            context_sources[id] = json!(if profile.is_some() { "bundled" } else { "unknown" });
            if let Some(profile) = profile {
                for key in crate::codex_context::FIELDS {
                    row[key] = profile[key].clone();
                }
                row["default_reasoning_level"] = profile["default"].clone();
                row["supported_reasoning_levels"] = Value::Array(
                    profile["levels"]
                        .as_array()
                        .ok_or("A built-in reasoning profile is invalid.")?
                        .iter()
                        .map(|effort| {
                            let label = effort.as_str().unwrap_or_default();
                            let known = reasoning_description(label);
                            json!({
                                "effort": effort,
                                "description": if known.is_empty() {
                                    format!("{label} reasoning effort")
                                } else {
                                    known.to_string()
                                }
                            })
                        })
                        .collect(),
                );
                row["supports_reasoning_summaries"] = json!(true);
                if !profile["multi_agent_reasoning_effort"].is_null() {
                    row["multi_agent_reasoning_effort"] = profile["multi_agent_reasoning_effort"].clone();
                }
            }

            let original = find_payload_model(payload, id).and_then(Value::as_object);
            let gateway_levels = original
                .is_some_and(|source| source.contains_key("supported_reasoning_levels"));
            let gateway_summaries = original
                .is_some_and(|source| source.contains_key("supports_reasoning_summaries"));
            let mut context_fields = Vec::new();
            if let Some(original) = original {
                for key in CODEX_MODEL_METADATA_FIELDS {
                    if let Some(value) = original.get(*key).filter(|value| !value.is_null()) {
                        row[*key] = value.clone();
                        if crate::codex_context::FIELDS.contains(key) {
                            context_fields.push(*key);
                        }
                    }
                }
                let gateway_context = original
                    .get("context_window")
                    .is_some_and(|value| !value.is_null());
                let gateway_max = original
                    .get("max_context_window")
                    .is_some_and(|value| !value.is_null());
                if gateway_context && !gateway_max {
                    row["max_context_window"] = row["context_window"].clone();
                    context_fields.push("max_context_window");
                } else if gateway_max && !gateway_context {
                    if let Some(maximum) = row["max_context_window"].as_i64() {
                        let context = row["context_window"]
                            .as_i64()
                            .map_or(maximum, |context| context.min(maximum));
                        row["context_window"] = json!(context);
                        context_fields.push("context_window");
                    }
                }
            }

            context_fields.sort_unstable();
            context_fields.dedup();
            if !context_fields.is_empty() {
                context_sources[id] = json!("gateway");
            }
            gateway_context_fields[id] = json!(context_fields);

            if !row["default_reasoning_level"].is_null() {
                effort_string(&mut row["default_reasoning_level"])?;
            }
            if !row["multi_agent_reasoning_effort"].is_null() {
                effort_string(&mut row["multi_agent_reasoning_effort"])?;
            }
            normalize_reasoning_levels(&mut row["supported_reasoning_levels"])?;
            if !row["supports_reasoning_summaries"].is_boolean() {
                return Err("The gateway reasoning capability flag must be boolean; no configuration was written.".into());
            }
            if gateway_levels && !gateway_summaries {
                row["supports_reasoning_summaries"] = json!(
                    row["supported_reasoning_levels"]
                        .as_array()
                        .is_some_and(|levels| levels.iter().any(|level| level["effort"] != "none"))
                );
            }
            if row["supports_reasoning_summaries"] == false
                && row["supported_reasoning_levels"]
                    .as_array()
                    .is_some_and(|levels| levels.iter().any(|level| level["effort"] != "none"))
            {
                return Err("The gateway reasoning levels conflict with its capability flag; Codex cannot send the selected effort. No configuration was written.".into());
            }
            let levels = row["supported_reasoning_levels"]
                .as_array()
                .ok_or("The generated reasoning levels are invalid.")?;
            let current_default = row["default_reasoning_level"].clone();
            let default_is_supported = !current_default.is_null()
                && levels
                    .iter()
                    .any(|level| level["effort"] == current_default);
            let fallback_default = levels
                    .iter()
                    .find(|level| level["effort"] == "medium")
                    .or_else(|| levels.first())
                    .map(|level| level["effort"].clone())
                    .unwrap_or(Value::Null);
            row["default_reasoning_level"] = if default_is_supported {
                current_default
            } else {
                fallback_default
            };
            normalize_catalog_shape(&mut row);
            crate::codex_context::validate(&row, false)?;
            row["slug"] = json!(id);
            row["visibility"] = json!("list");
            row["supported_in_api"] = json!(true);
            row["priority"] = json!(index);
            row["upgrade"] = Value::Null;
            Ok(row)
        })
        .collect::<Result<_, String>>()?;
    serde_json::to_string_pretty(&json!({
        "models":entries,
        "_tokentoapi_connect": {
            "context_sources": context_sources,
            "gateway_context_fields": gateway_context_fields
        }
    }))
    .map_err(|_| "Could not generate the model catalog.".into())
}

#[cfg(test)]
mod session_tests {
    use super::*;
    #[test]
    fn expired_discovery_requires_a_new_fetch_even_with_the_matching_id() {
        let mut session = DiscoverySession::new(DiscoveredModels {
            client: Client::Claude,
            api_key: "fake-only".into(),
            root: "https://example.test".into(),
            payload: json!({}),
            choices: vec![ModelChoice {
                id: "a".into(),
                name: "a".into(),
            }],
        });
        let input = SelectionInput {
            discovery_id: session.response().discovery_id,
            selected_models: vec!["a".into()],
            default_model: "a".into(),
            options: Default::default(),
        };
        assert!(session.prepare(&input).is_ok());
        session.created = Instant::now() - Duration::from_secs(901);
        assert!(session.prepare(&input).is_err());
    }
}
