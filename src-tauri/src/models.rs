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
        prepare_selection(&self.data, &input.selected_models, &input.default_model)
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
                if !id.is_empty()
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

const KNOWN_CODEX_REASONING_MODELS: &[&str] =
    &["gpt-5.5", "gpt-5.6-sol", "gpt-5.6-terra", "gpt-6-astra"];
const CODEX_MODEL_METADATA_FIELDS: &[&str] = &[
    "display_name",
    "description",
    "context_window",
    "max_context_window",
    "effective_context_window_percent",
    "input_modalities",
];
const VALID_REASONING_EFFORTS: &[&str] = &["none", "minimal", "low", "medium", "high", "xhigh"];
const STANDARD_CODEX_REASONING_EFFORTS: &[&str] = &["low", "medium", "high", "xhigh"];

fn is_reasoning_effort(value: &str) -> bool {
    VALID_REASONING_EFFORTS.contains(&value)
}

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

fn reasoning_level(effort: &str, description: Option<&str>) -> Value {
    json!({
        "effort": effort,
        "description": description
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| reasoning_description(effort)),
    })
}

// Gateways commonly expose levels as strings, while Codex expects objects. Invalid
// advertised values deliberately result in no options instead of an unsafe fallback.
fn declared_reasoning_levels(value: Option<&Value>) -> Option<Vec<Value>> {
    let value = value?;
    let Some(values) = value.as_array() else {
        return Some(Vec::new());
    };
    let mut levels = Vec::new();
    for value in values {
        let (effort, description) = match value {
            Value::String(effort) => (effort.as_str(), None),
            Value::Object(object) => (
                object
                    .get("effort")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                object.get("description").and_then(Value::as_str),
            ),
            _ => continue,
        };
        if is_reasoning_effort(effort)
            && !levels.iter().any(|level: &Value| level["effort"] == effort)
        {
            levels.push(reasoning_level(effort, description));
        }
    }
    Some(levels)
}

fn reasoning_capabilities(
    id: &str,
    original: Option<&serde_json::Map<String, Value>>,
) -> (Value, Value) {
    let declared =
        original.and_then(|row| declared_reasoning_levels(row.get("supported_reasoning_levels")));
    let levels = declared.unwrap_or_else(|| {
        if KNOWN_CODEX_REASONING_MODELS
            .iter()
            .any(|known| known.eq_ignore_ascii_case(id))
        {
            STANDARD_CODEX_REASONING_EFFORTS
                .iter()
                .map(|effort| reasoning_level(effort, None))
                .collect()
        } else {
            Vec::new()
        }
    });
    let default = original
        .and_then(|row| row.get("default_reasoning_level"))
        .and_then(Value::as_str)
        .filter(|effort| levels.iter().any(|level| level["effort"] == *effort))
        .or_else(|| {
            levels
                .iter()
                .find(|level| level["effort"] == "medium")
                .and_then(|level| level["effort"].as_str())
        })
        .or_else(|| levels.first().and_then(|level| level["effort"].as_str()));
    (
        default.map_or(Value::Null, |effort| json!(effort)),
        Value::Array(levels),
    )
}

fn bounded_u64(value: Option<&Value>, minimum: u64, maximum: u64) -> Option<u64> {
    value
        .and_then(Value::as_u64)
        .filter(|value| (minimum..=maximum).contains(value))
}

fn normalize_catalog_shape(row: &mut Value) {
    let context_window =
        bounded_u64(row.get("context_window"), 1024, 16 * 1024 * 1024).unwrap_or(32768);
    row["context_window"] = json!(context_window);
    let max_context_window = bounded_u64(row.get("max_context_window"), 1024, 16 * 1024 * 1024)
        .unwrap_or(context_window)
        .max(context_window);
    row["max_context_window"] = json!(max_context_window);
    let percent = bounded_u64(row.get("effective_context_window_percent"), 1, 100).unwrap_or(95);
    row["effective_context_window_percent"] = json!(percent);
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
    // The credential is only sent to this exact origin. No redirect or response body is echoed.
    let mut response = client
        .get(format!("{root}/v1/models"))
        .bearer_auth(&api_key)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                "Connection timed out; check your network and the API address."
            } else {
                "Could not reach the API; check your network, certificates and the API address."
            }
            .to_string()
        })?;
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
        if !available.contains(id.as_str()) || !seen.insert(id) {
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
    })
}

// CC Switch's native Responses catalog template; gateway-supplied metadata takes precedence.
// The selected list is the complete external catalog, so unchecked models never leak into it.
pub fn build_codex_catalog(payload: &Value, selected: &[String]) -> Result<String, String> {
    let template: Value = serde_json::from_str(include_str!(
        "resources/codex_native_responses_template.json"
    ))
    .map_err(|_| "The model catalog template is unavailable.")?;
    let source = model_rows(payload);
    let entries: Vec<Value> = selected
        .iter()
        .enumerate()
        .map(|(index, id)| {
            let mut row = template.clone();
            // Names alone do not declare capabilities, including a slug-only native catalog.
            row["context_window"] = json!(32768);
            row["max_context_window"] = json!(32768);
            row["input_modalities"] = json!(["text"]);
            row["supports_reasoning_summaries"] = json!(false);
            let original = source
                .and_then(|rows| {
                    rows.iter().find(|model| {
                        model_identifier(model).is_some_and(|candidate| candidate.trim() == id)
                    })
                })
                .and_then(Value::as_object);
            if let Some(original) = original {
                for key in CODEX_MODEL_METADATA_FIELDS {
                    if let Some(value) = original.get(*key) {
                        row[*key] = value.clone();
                    }
                }
            }
            normalize_catalog_shape(&mut row);
            let (default_reasoning_level, supported_reasoning_levels) =
                reasoning_capabilities(id, original);
            row["default_reasoning_level"] = default_reasoning_level;
            row["supported_reasoning_levels"] = supported_reasoning_levels;
            row["slug"] = json!(id);
            if source.is_none() || row["display_name"] == template["display_name"] {
                row["display_name"] = json!(id);
                row["description"] = json!(id);
            }
            row["visibility"] = json!("list");
            row["supported_in_api"] = json!(true);
            row["priority"] = json!(index);
            row["upgrade"] = Value::Null;
            row
        })
        .collect();
    serde_json::to_string_pretty(&json!({"models":entries}))
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
        };
        assert!(session.prepare(&input).is_ok());
        session.created = Instant::now() - Duration::from_secs(901);
        assert!(session.prepare(&input).is_err());
    }
}
