//! Import options that are validated once and shared by every client writer.
use crate::models::{Client, ValidatedInput};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactMode {
    #[default]
    Preserve,
    Disabled,
    Enabled,
}

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WebSearchMode {
    #[default]
    Preserve,
    Disabled,
    Cached,
    Live,
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupOptions {
    #[serde(default)]
    pub claude_models: BTreeMap<String, String>,
    #[serde(default)]
    pub artifact_mode: ArtifactMode,
    #[serde(default)]
    pub web_search: WebSearchMode,
}

impl SetupOptions {
    pub fn web_search_value(&self, doc: &toml_edit::DocumentMut) -> Result<&'static str, String> {
        Ok(match self.web_search {
            WebSearchMode::Disabled => "disabled",
            WebSearchMode::Cached => "cached",
            WebSearchMode::Live => "live",
            WebSearchMode::Preserve => {
                match doc.get("web_search").and_then(toml_edit::Item::as_str) {
                    None if !doc.contains_key("web_search") => "disabled",
                    Some("disabled") => "disabled",
                    Some("cached") => "cached",
                    Some("live") => "live",
                    _ => {
                        return Err("The existing web_search setting is invalid. Choose disabled, cached or live before importing.".into());
                    }
                }
            }
        })
    }

    pub fn validate(&self, input: &ValidatedInput) -> Result<(), String> {
        for (role, model) in &self.claude_models {
            if !["haiku", "sonnet", "opus", "fable", "subagent"].contains(&role.as_str())
                || !input.selected_models.contains(model)
            {
                return Err(
                    "Claude purpose models must be selected models; refresh and select them again."
                        .into(),
                );
            }
        }
        if input.client.is_codex() {
            if !self.claude_models.is_empty() || self.artifact_mode != ArtifactMode::Preserve {
                return Err("Codex does not accept Claude-specific setup options.".into());
            }
        } else if self.web_search != WebSearchMode::Preserve {
            return Err("The web search option is only available for Codex.".into());
        }
        if matches!(input.client, Client::ClaudeDesktop | Client::Grok)
            && (!self.claude_models.is_empty() || self.artifact_mode != ArtifactMode::Preserve)
        {
            return Err(
                "This client does not use Claude Code purpose models or Artifact settings.".into(),
            );
        }
        Ok(())
    }
}
