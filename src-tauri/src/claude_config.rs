//! Claude Code provider environment shared by the terminal and VS Code writers.
use crate::{
    models::ValidatedInput,
    provider_fields,
    setup_options::{ArtifactMode, ToolSearchMode},
};
use std::collections::BTreeMap;

pub fn role_models(input: &ValidatedInput) -> BTreeMap<String, String> {
    let mut roles = BTreeMap::new();
    for role in ["haiku", "sonnet", "opus", "fable"] {
        let model = input
            .options
            .claude_models
            .get(role)
            .cloned()
            .unwrap_or_else(|| {
                let family = format!("claude-{role}-");
                let matches: Vec<_> = input
                    .selected_models
                    .iter()
                    .filter(|id| {
                        id.strip_prefix("anthropic/")
                            .unwrap_or(id)
                            .to_ascii_lowercase()
                            .starts_with(&family)
                    })
                    .collect();
                if matches.iter().any(|id| **id == input.model) {
                    input.model.clone()
                } else if matches.len() == 1 {
                    matches[0].clone()
                } else {
                    input.model.clone()
                }
            });
        roles.insert(role.into(), model);
    }
    roles.insert(
        "subagent".into(),
        input
            .options
            .claude_models
            .get("subagent")
            .cloned()
            .unwrap_or_default(),
    );
    roles
}

pub fn environment(input: &ValidatedInput) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    // Settings-file cleanup alone cannot override credentials inherited by the client process.
    for key in provider_fields::CLAUDE_PROTOCOL_SELECTORS
        .iter()
        .copied()
        .chain([
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_MODEL",
            "ANTHROPIC_CUSTOM_HEADERS",
            "ANTHROPIC_SMALL_FAST_MODEL",
            "CLAUDE_CODE_OAUTH_TOKEN",
            "CLAUDE_CODE_OAUTH_REFRESH_TOKEN",
            "CLAUDE_CODE_OAUTH_SCOPES",
            "CLAUDE_CODE_SUBAGENT_MODEL",
            "CLAUDE_CODE_SUBAGENT_MODEL_FORCE",
            "CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS",
            "CLAUDE_CODE_DISABLE_THINKING",
            "DISABLE_INTERLEAVED_THINKING",
            "CLAUDE_CODE_ALWAYS_ENABLE_EFFORT",
            "CLAUDE_CODE_MAX_CONTEXT_TOKENS",
            "CLAUDE_CODE_AUTO_COMPACT_WINDOW",
            "CLAUDE_CODE_MAX_OUTPUT_TOKENS",
            "CLAUDE_CODE_DISABLE_1M_CONTEXT",
            "CLAUDE_CODE_DISABLE_UNKNOWN_MODEL_WINDOW_ENFORCEMENT",
        ])
    {
        env.insert(key.into(), String::new());
    }
    for (key, value) in [
        ("ANTHROPIC_BASE_URL", input.root.as_str()),
        ("ANTHROPIC_AUTH_TOKEN", input.api_key.as_str()),
        ("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1"),
        ("CLAUDE_CODE_AUTO_MODE_SERVER", "0"),
    ] {
        env.insert(key.into(), value.into());
    }
    match input.options.tool_search {
        ToolSearchMode::Preserve => {}
        ToolSearchMode::Disabled => {
            env.insert("ENABLE_TOOL_SEARCH".into(), "false".into());
        }
        ToolSearchMode::Enabled => {
            env.insert("ENABLE_TOOL_SEARCH".into(), "true".into());
        }
    }
    for (role, model) in role_models(input) {
        if role == "subagent" && model.is_empty() {
            continue;
        }
        let key = if role == "subagent" {
            "CLAUDE_CODE_SUBAGENT_MODEL".to_string()
        } else {
            format!("ANTHROPIC_DEFAULT_{}_MODEL", role.to_uppercase())
        };
        env.insert(key, model);
    }
    match input.options.artifact_mode {
        ArtifactMode::Preserve => {}
        ArtifactMode::Disabled => {
            env.insert("CLAUDE_CODE_DISABLE_ARTIFACT".into(), "1".into());
        }
        ArtifactMode::Enabled => {
            env.insert("CLAUDE_CODE_DISABLE_ARTIFACT".into(), "0".into());
        }
    }
    env
}
