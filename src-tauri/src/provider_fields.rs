// Provider-field classification extracted from CC Switch, MIT. See UPSTREAM.md.
//! Definitions of key fields (the "floor") and provider-exclusive fields.
//!
//! Key fields answer four questions: where the request goes, how it authenticates,
//! which model name, and which protocol. They belong entirely to the provider:
//! switching clears them and writes the target provider values; all other keys
//! belong to the user and the client, and CC Switch neither writes nor removes them.
//!
//! Prefix matching is used only where the whole prefix belongs to connection and
//! authentication (`ANTHROPIC_*`, `AWS_*`, Gemini’s `GOOGLE_*`). Missed keys are
//! kept as user keys untouched, so failure is safe.
//!
//! Provider-exclusive fields are upstream compatibility switches and window
//! values: written on switch-in, and on switch-away only removed when they were
//! brought in by the previous provider and their values were not changed. They are
//! not key fields, because users may also set them globally.

/// Claude Code protocol selectors.
///
/// `CLAUDE_CODE_USE_` cannot be matched by prefix: the same prefix also contains
/// provider-agnostic feature switches such as `USE_POWERSHELL_TOOL`,
/// `USE_NATIVE_FILE_SEARCH`, `USE_COWORK_PLUGINS` and `USE_CCR_V2` (verified with
/// Claude Code 2.1.282). Any `CLAUDE_CODE_USE_*` present in presets must appear here.
pub const CLAUDE_PROTOCOL_SELECTORS: &[&str] = &[
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "CLAUDE_CODE_USE_FOUNDRY",
    "CLAUDE_CODE_USE_GATEWAY",
    "CLAUDE_CODE_USE_MANTLE",
    "CLAUDE_CODE_USE_ANTHROPIC_AWS",
    "CLAUDE_CODE_USE_ANTHROPIC_GOOGLE_CLOUD",
];

/// Prefixes in `env` whose entire prefix belongs to connection and authentication:
/// `ANTHROPIC_*` covers the address, credentials, tier model names and custom
/// headers; `AWS_*` covers Bedrock regions and credentials; `VERTEX_REGION_*`
/// covers per-model Vertex regions.
pub const CLAUDE_FLOOR_ENV_PREFIXES: &[&str] = &["ANTHROPIC_", "AWS_", "VERTEX_REGION_"];

/// Key fields in `env` listed by name (besides protocol selectors).
pub const CLAUDE_FLOOR_ENV_KEYS: &[&str] = &[
    "CLAUDE_CODE_SUBAGENT_MODEL",
    "CLAUDE_CODE_SUBAGENT_MODEL_FORCE",
    "CLOUD_ML_REGION",
    // Vertex credentials path.
    "GOOGLE_APPLICATION_CREDENTIALS",
    // Long-lived tokens for subscription accounts and their companion keys.
    "CLAUDE_CODE_OAUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_REFRESH_TOKEN",
    "CLAUDE_CODE_OAUTH_SCOPES",
    // Companion to the top-level apiKeyHelper.
    "CLAUDE_CODE_API_KEY_HELPER_TTL_MS",
];

/// Whether a key in the `env` of Claude Code `settings.json` is a key field.
///
/// The frontend preset scan (`tests/config/claudeKeyFields.json`) mirrors this,
/// and the tests below keep the two in sync.
pub fn claude_floor_env(key: &str) -> bool {
    CLAUDE_FLOOR_ENV_PREFIXES
        .iter()
        .any(|prefix| key.starts_with(prefix))
        || CLAUDE_PROTOCOL_SELECTORS.contains(&key)
        || CLAUDE_FLOOR_ENV_KEYS.contains(&key)
        || (key.starts_with("CLAUDE_CODE_SKIP_") && key.ends_with("_AUTH"))
}

/// Key fields at the top level of Claude Code `settings.json`.
pub const CLAUDE_FLOOR_TOP: &[&str] = &[
    "apiKeyHelper",
    "apiBaseUrl",
    "primaryModel",
    "smallFastModel",
    // Old Bedrock API Key presets wrote the real key at the top level.
    "apiKey",
    // The choice saved by `/model` belongs to the provider active at the time.
    "model",
    // Fallback model chain; model ID -> provider-specific ID (e.g. Bedrock ARN).
    "fallbackModel",
    "modelOverrides",
    // `/model` picker rows: Stack models listed by CC Switch in aggregate mode,
    // otherwise not kept.
    "modelPicker",
    // advisor is only available on the Anthropic API.
    "advisorModel",
    // Bedrock / Vertex credential commands.
    "awsAuthRefresh",
    "awsCredentialExport",
    "gcpAuthRefresh",
];

pub fn claude_floor_top(key: &str) -> bool {
    CLAUDE_FLOOR_TOP.contains(&key)
}

/// Claude Code provider-exclusive fields (in `env`).
pub const CLAUDE_EXCLUSIVE_ENV: &[&str] = &[
    // AtlasCloud, Soshow, etc. reject experimental beta headers.
    "CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS",
    // DeepSeek and others strictly validate tool schemas; the Artifact tool makes
    // every request return 400.
    "CLAUDE_CODE_DISABLE_ARTIFACT",
    // Off by default under third-party addresses; only enabled when the upstream
    // forwards tool_reference blocks.
    "ENABLE_TOOL_SEARCH",
    // Compatibility options documented for proxies, gateways and third parties.
    "CLAUDE_CODE_DISABLE_THINKING",
    "DISABLE_INTERLEAVED_THINKING",
    "CLAUDE_CODE_ALWAYS_ENABLE_EFFORT",
    "CLAUDE_CODE_EXTRA_BODY",
    "CLAUDE_CODE_ENABLE_FINE_GRAINED_TOOL_STREAMING",
    // The auto-mode server-side classifier is only supported on official endpoints;
    // set 0 for gateway scenarios, otherwise sessions stall on blocking prompts
    // (documented compatibility option for proxies and gateways).
    "CLAUDE_CODE_AUTO_MODE_SERVER",
    // Window values: determined by the upstream model window.
    "CLAUDE_CODE_MAX_CONTEXT_TOKENS",
    "CLAUDE_CODE_AUTO_COMPACT_WINDOW",
    "CLAUDE_CODE_MAX_OUTPUT_TOKENS",
    "CLAUDE_CODE_DISABLE_1M_CONTEXT",
    "CLAUDE_CODE_DISABLE_UNKNOWN_MODEL_WINDOW_ENFORCEMENT",
    // Fetches the model list from ANTHROPIC_BASE_URL: gateways (including Stack
    // models in proxy mode) need it, and users may also set it globally.
    "CLAUDE_CODE_ENABLE_GATEWAY_MODEL_DISCOVERY",
];

pub fn claude_exclusive_env(key: &str) -> bool {
    CLAUDE_EXCLUSIVE_ENV.contains(&key)
}

/// Key fields at the top level of the Codex `config.toml`. There is also the
/// whole `[model_providers.custom]` table.
pub const CODEX_FLOOR_TOP: &[&str] = &[
    // Routing: `openai_base_url` reroutes the built-in openai route elsewhere.
    "model_provider",
    "openai_base_url",
    "model",
    "review_model",
    "model_reasoning_effort",
    "plan_mode_reasoning_effort",
    "disable_response_storage",
    "model_catalog_json",
    // Fallback spelling writes the key at the top level.
    "experimental_bearer_token",
    // Legacy form: lands at the top level when there is no model_provider.
    "base_url",
    "wire_api",
];

/// Model names nested in the user’s own tables: only these keys are cleared;
/// other keys in the tables are untouched.
pub const CODEX_FLOOR_NESTED: &[&[&str]] = &[
    &["agents", "default_subagent_model"],
    &["agents", "default_subagent_reasoning_effort"],
    &["memories", "extract_model"],
    &["memories", "consolidation_model"],
];

/// The provider table CC Switch writes into the Codex live config.
pub const CODEX_PROVIDER_TABLE: &[&str] = &["model_providers", "custom"];
