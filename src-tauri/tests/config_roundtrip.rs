use std::fs;
use tokentoapi_connect_lib::{
    adapter,
    models::{self, Client, ValidatedInput},
    restore,
    setup_options::{ArtifactMode, WebSearchMode},
    storage::{self, Change, FileKind, Paths},
};

fn input(client: Client) -> ValidatedInput {
    ValidatedInput {
        client,
        api_key: "sk-fake-test-only".into(),
        root: "https://api.example.test".into(),
        model: match client {
            Client::Claude | Client::ClaudeDesktop | Client::ClaudeVscode => "claude-sonnet-test",
            Client::Codex | Client::CodexDesktop => "gpt-test-codex",
        }
        .into(),
        selected_models: vec![match client {
            Client::Claude | Client::ClaudeDesktop | Client::ClaudeVscode => "claude-sonnet-test",
            Client::Codex | Client::CodexDesktop => "gpt-test-codex",
        }
        .into()],
        catalog: None,
        options: Default::default(),
    }
}

#[test]
fn codex_merge_keeps_comments_mcp_and_permissions_and_restores_login_exactly() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    let original="# My comment\nmodel = \"old\" # model note\nmodel_reasoning_effort = \"medium\"\nplan_mode_reasoning_effort = \"xhigh\"\napproval_policy = \"on-request\"\ncli_auth_credentials_store = \"keyring\"\nweb_search = \"live\"\n\n[mcp_servers.example]\ncommand = \"my-server\"\n\n[model_providers.original]\nname = \"Original\"\n";
    let auth = "{\n  \"tokens\": {\"refresh_token\": \"fake-test-oauth\"}\n}\n";
    storage::atomic_write(&paths.target(FileKind::CodexConfig), original.as_bytes()).unwrap();
    storage::atomic_write(&paths.target(FileKind::CodexAuth), auth.as_bytes()).unwrap();
    let mut imported = input(Client::Codex);
    imported.catalog = Some(
        models::build_codex_catalog(
            &serde_json::json!({"data":[{"id":"gpt-test-codex","context_window":128000}]}),
            &imported.selected_models,
        )
        .unwrap(),
    );
    let result = adapter::apply(&paths, imported).unwrap();
    assert!(result.changed);
    let merged = fs::read_to_string(paths.target(FileKind::CodexConfig)).unwrap();
    assert!(merged.contains("# My comment"));
    assert!(merged.contains("# model note"));
    assert!(merged.contains("command = \"my-server\""));
    assert!(merged.contains("approval_policy = \"on-request\""));
    assert!(merged.contains("[model_providers.original]"));
    let doc = merged.parse::<toml_edit::DocumentMut>().unwrap();
    assert_eq!(doc["model_provider"].as_str(), Some("tokentoapi-connect"));
    assert!(doc.get("model_reasoning_effort").is_none());
    assert!(doc.get("plan_mode_reasoning_effort").is_none());
    assert_eq!(
        doc["model_catalog_json"].as_str(),
        Some(
            paths
                .target(FileKind::CodexCatalog)
                .to_string_lossy()
                .as_ref()
        )
    );
    assert_eq!(
        doc["model_providers"]["tokentoapi-connect"]["base_url"].as_str(),
        Some("https://api.example.test/v1")
    );
    assert_eq!(
        doc["model_providers"]["tokentoapi-connect"]["experimental_bearer_token"].as_str(),
        Some("sk-fake-test-only")
    );
    assert_eq!(
        doc["model_providers"]["tokentoapi-connect"]["requires_openai_auth"].as_bool(),
        Some(false)
    );
    assert_eq!(doc["web_search"].as_str(), Some("live"));
    assert_eq!(doc["cli_auth_credentials_store"].as_str(), Some("keyring"));
    assert_eq!(
        fs::read_to_string(paths.target(FileKind::CodexAuth)).unwrap(),
        auth
    );
    let encoded = serde_json::to_string(&result).unwrap();
    assert!(!encoded.contains("sk-fake-test-only"));
    storage::restore(&paths, Client::Codex).unwrap();
    assert_eq!(
        fs::read_to_string(paths.target(FileKind::CodexConfig)).unwrap(),
        original
    );
    assert_eq!(
        fs::read_to_string(paths.target(FileKind::CodexAuth)).unwrap(),
        auth
    );
}

#[test]
fn new_install_and_repeated_import_keep_the_original_restore_point() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    adapter::apply(&paths, input(Client::Codex)).unwrap();
    let backup = fs::read(paths.state.join("codex.backup.json")).unwrap();
    assert!(
        !adapter::apply(&paths, input(Client::Codex))
            .unwrap()
            .changed
    );
    assert_eq!(
        backup,
        fs::read(paths.state.join("codex.backup.json")).unwrap()
    );
    storage::restore(&paths, Client::Codex).unwrap();
    assert!(!paths.target(FileKind::CodexAuth).exists());
    assert!(!paths.target(FileKind::CodexConfig).exists());
}

#[test]
fn claude_merge_preserves_hooks_permissions_unrelated_env_and_bom() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    let original="\u{feff}{\r\n\t\"hooks\": {\"Stop\": []},\r\n\t\"permissions\": {\"allow\": [\"Read\"]},\r\n\t\"env\": {\"CUSTOM_VAR\": \"keep\", \"ANTHROPIC_API_KEY\": \"old-test\", \"CLAUDE_CODE_USE_BEDROCK\": \"1\", \"CLAUDE_CODE_DISABLE_THINKING\": \"1\", \"DISABLE_INTERLEAVED_THINKING\": \"1\", \"CLAUDE_CODE_ALWAYS_ENABLE_EFFORT\": \"1\", \"CLAUDE_CODE_MAX_CONTEXT_TOKENS\": \"8192\"}\r\n}\r\n";
    storage::atomic_write(&paths.target(FileKind::ClaudeSettings), original.as_bytes()).unwrap();
    adapter::apply(&paths, input(Client::Claude)).unwrap();
    let merged = fs::read_to_string(paths.target(FileKind::ClaudeSettings)).unwrap();
    assert!(merged.starts_with('\u{feff}'));
    assert!(merged.contains("\r\n\t"));
    let doc: serde_json::Value =
        serde_json::from_str(merged.trim_start_matches('\u{feff}')).unwrap();
    assert_eq!(doc["hooks"]["Stop"], serde_json::json!([]));
    assert_eq!(doc["permissions"]["allow"], serde_json::json!(["Read"]));
    assert_eq!(doc["env"]["CUSTOM_VAR"], "keep");
    assert_eq!(doc["env"]["ANTHROPIC_API_KEY"], "");
    assert_eq!(doc["env"]["CLAUDE_CODE_USE_BEDROCK"], "");
    for key in [
        "CLAUDE_CODE_DISABLE_THINKING",
        "DISABLE_INTERLEAVED_THINKING",
        "CLAUDE_CODE_ALWAYS_ENABLE_EFFORT",
        "CLAUDE_CODE_MAX_CONTEXT_TOKENS",
    ] {
        assert_eq!(doc["env"][key], "", "{key} was not neutralized");
    }
    storage::restore(&paths, Client::Claude).unwrap();
    assert_eq!(
        fs::read_to_string(paths.target(FileKind::ClaudeSettings)).unwrap(),
        original
    );
}

#[test]
fn malformed_configs_and_active_profiles_make_no_client_changes() {
    for (client, kind, text) in [
        (Client::Claude, FileKind::ClaudeSettings, "{broken"),
        (Client::Claude, FileKind::ClaudeSettings, "{\"env\":42}"),
        (Client::Codex, FileKind::CodexConfig, "model = ["),
        (Client::Codex, FileKind::CodexConfig, "profile = \"work\"\n"),
    ] {
        let home = tempfile::tempdir().unwrap();
        let paths = Paths::for_home(home.path());
        storage::atomic_write(&paths.target(kind), text.as_bytes()).unwrap();
        assert!(adapter::apply(&paths, input(client)).is_err());
        assert_eq!(fs::read_to_string(paths.target(kind)).unwrap(), text);
        assert!(!paths.has_backup(client));
        assert!(!paths.target(FileKind::CodexAuth).exists());
    }
}

#[test]
fn restore_refuses_to_destroy_later_user_edits_or_a_different_config_directory() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    adapter::apply(&paths, input(Client::Codex)).unwrap();
    let mut redirected = paths.clone();
    redirected.codex = home.path().join("other-codex");
    assert!(storage::restore(&redirected, Client::Codex)
        .unwrap_err()
        .contains("directory has changed"));
    let config = paths.target(FileKind::CodexConfig);
    let mut text = fs::read_to_string(&config).unwrap();
    text.push_str("\n# My later edit\n");
    storage::atomic_write(&config, text.as_bytes()).unwrap();
    let auth = storage::read_optional(&paths.target(FileKind::CodexAuth)).unwrap();
    assert!(storage::restore(&paths, Client::Codex)
        .unwrap_err()
        .contains("modified again"));
    assert_eq!(fs::read_to_string(&config).unwrap(), text);
    assert_eq!(
        storage::read_optional(&paths.target(FileKind::CodexAuth)).unwrap(),
        auth
    );
    assert!(paths.has_backup(Client::Codex));
}

#[test]
fn optional_codex_catalog_is_installed_and_restored_with_the_config() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    let mut data = input(Client::Codex);
    data.catalog = Some(
        models::build_codex_catalog(
            &serde_json::json!({"data":[{"id":"gpt-test-codex","context_window":128000}]}),
            &data.selected_models,
        )
        .unwrap(),
    );
    adapter::apply(&paths, data).unwrap();
    assert!(paths.target(FileKind::CodexCatalog).is_file());
    let config = fs::read_to_string(paths.target(FileKind::CodexConfig)).unwrap();
    assert!(config.contains("model_catalog_json"));
    storage::restore(&paths, Client::Codex).unwrap();
    assert!(!paths.target(FileKind::CodexCatalog).exists());
}

#[cfg(unix)]
#[test]
fn private_permissions_and_symlink_protection() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    adapter::apply(&paths, input(Client::Claude)).unwrap();
    for path in [
        paths.target(FileKind::ClaudeSettings),
        paths.state.join("claude.backup.json"),
    ] {
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    storage::restore(&paths, Client::Claude).unwrap();
    let outside = home.path().join("important.json");
    fs::write(&outside, "{}").unwrap();
    symlink(&outside, paths.target(FileKind::ClaudeSettings)).unwrap();
    assert!(adapter::apply(&paths, input(Client::Claude))
        .unwrap_err()
        .contains("symbolic link"));
    assert_eq!(fs::read_to_string(&outside).unwrap(), "{}");
}

#[test]
fn claude_picker_contains_exactly_selected_models_and_default_can_be_changed_later() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    let original = r#"{"model":"old","modelPicker":{"options":[{"model":"old"}]},"availableModels":["old"],"env":{"ANTHROPIC_MODEL":"old"}}"#;
    storage::atomic_write(&paths.target(FileKind::ClaudeSettings), original.as_bytes()).unwrap();
    let mut selected = input(Client::Claude);
    selected.selected_models = vec!["claude-sonnet-test".into(), "custom-team-model".into()];
    selected.model = "custom-team-model".into();
    adapter::apply(&paths, selected).unwrap();
    let result: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(paths.target(FileKind::ClaudeSettings)).unwrap())
            .unwrap();
    assert_eq!(result["model"], "custom-team-model");
    assert_eq!(
        result["availableModels"],
        serde_json::json!(["claude-sonnet-test", "custom-team-model"])
    );
    let picker = &result["modelPicker"];
    assert_eq!(picker["replaceBuiltInOptions"], true);
    assert_eq!(picker["options"].as_array().unwrap().len(), 2);
    assert_eq!(picker["options"][1]["model"], "custom-team-model");
    assert_eq!(result["env"]["ANTHROPIC_MODEL"], "");
    storage::restore(&paths, Client::Claude).unwrap();
    assert_eq!(
        fs::read_to_string(paths.target(FileKind::ClaudeSettings)).unwrap(),
        original
    );
}

#[test]
fn legacy_codex_auth_import_migrates_without_losing_the_original_restore_point() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    let original_config = "model = \"official\"\ncli_auth_credentials_store = \"keyring\"\n";
    let original_auth = "{\"tokens\":{\"refresh_token\":\"original-oauth\"}}\n";
    let legacy_auth = "{\"OPENAI_API_KEY\":\"old-connect-key\"}\n";
    let legacy_config = "model = \"old\"\n";
    storage::atomic_write(
        &paths.target(FileKind::CodexConfig),
        original_config.as_bytes(),
    )
    .unwrap();
    storage::atomic_write(&paths.target(FileKind::CodexAuth), original_auth.as_bytes()).unwrap();
    storage::commit(
        &paths,
        Client::Codex,
        vec![
            Change {
                kind: FileKind::CodexAuth,
                before: Some(original_auth.into()),
                after: legacy_auth.into(),
            },
            Change {
                kind: FileKind::CodexConfig,
                before: Some(original_config.into()),
                after: legacy_config.into(),
            },
        ],
    )
    .unwrap();
    let mut next = input(Client::Codex);
    next.options.web_search = WebSearchMode::Cached;
    adapter::apply(&paths, next).unwrap();

    assert_eq!(
        fs::read_to_string(paths.target(FileKind::CodexAuth)).unwrap(),
        legacy_auth
    );
    let config = fs::read_to_string(paths.target(FileKind::CodexConfig)).unwrap();
    let parsed = config.parse::<toml_edit::DocumentMut>().unwrap();
    assert_eq!(parsed["web_search"].as_str(), Some("cached"));
    assert_eq!(
        parsed["model_providers"]["tokentoapi-connect"]["experimental_bearer_token"].as_str(),
        Some("sk-fake-test-only")
    );
    storage::restore(&paths, Client::Codex).unwrap();
    assert_eq!(
        fs::read_to_string(paths.target(FileKind::CodexConfig)).unwrap(),
        original_config
    );
    assert_eq!(
        fs::read_to_string(paths.target(FileKind::CodexAuth)).unwrap(),
        original_auth
    );
}

#[test]
fn codex_reserved_provider_names_are_removed_and_restored_byte_for_byte() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    let original = "[model_providers.openai]\nname = \"wrong\"\n\n[model_providers.ollama]\nname = \"local\"\n\n[model_providers.custom]\nname = \"keep\"\n";
    storage::atomic_write(&paths.target(FileKind::CodexConfig), original.as_bytes()).unwrap();
    adapter::apply(&paths, input(Client::Codex)).unwrap();
    let config = fs::read_to_string(paths.target(FileKind::CodexConfig)).unwrap();
    let parsed = config.parse::<toml_edit::DocumentMut>().unwrap();
    assert!(parsed["model_providers"].get("openai").is_none());
    assert!(parsed["model_providers"].get("ollama").is_none());
    assert!(parsed["model_providers"].get("custom").is_some());
    storage::restore(&paths, Client::Codex).unwrap();
    assert_eq!(
        fs::read_to_string(paths.target(FileKind::CodexConfig)).unwrap(),
        original
    );
}

#[test]
fn claude_role_models_artifact_preference_and_inherited_environment_are_handled() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    let original = r#"{"env":{"CLAUDE_CODE_DISABLE_ARTIFACT":"1","ANTHROPIC_API_KEY":"old","CLAUDE_CODE_USE_BEDROCK":"1"}}"#;
    storage::atomic_write(&paths.target(FileKind::ClaudeSettings), original.as_bytes()).unwrap();
    let mut selected = input(Client::Claude);
    selected.model = "claude-sonnet-4-6".into();
    selected.selected_models = vec![
        "claude-sonnet-4-6".into(),
        "claude-haiku-4-5".into(),
        "claude-opus-4-6".into(),
    ];
    adapter::apply(&paths, selected).unwrap();
    let text = fs::read_to_string(paths.target(FileKind::ClaudeSettings)).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        parsed["env"]["ANTHROPIC_DEFAULT_HAIKU_MODEL"],
        "claude-haiku-4-5"
    );
    assert_eq!(
        parsed["env"]["ANTHROPIC_DEFAULT_OPUS_MODEL"],
        "claude-opus-4-6"
    );
    assert_eq!(
        parsed["env"]["ANTHROPIC_DEFAULT_SONNET_MODEL"],
        "claude-sonnet-4-6"
    );
    assert_eq!(parsed["env"]["CLAUDE_CODE_DISABLE_ARTIFACT"], "1");
    assert_eq!(parsed["env"]["ANTHROPIC_API_KEY"], "");
    assert_eq!(parsed["env"]["CLAUDE_CODE_USE_BEDROCK"], "");
    assert_eq!(parsed["env"]["CLAUDE_CODE_AUTO_MODE_SERVER"], "0");
    let mut enabled = input(Client::Claude);
    enabled.model = "claude-sonnet-4-6".into();
    enabled.selected_models = vec![
        "claude-sonnet-4-6".into(),
        "claude-haiku-4-5".into(),
        "claude-opus-4-6".into(),
    ];
    enabled.options.artifact_mode = ArtifactMode::Enabled;
    adapter::apply(&paths, enabled).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(paths.target(FileKind::ClaudeSettings)).unwrap())
            .unwrap();
    assert_eq!(parsed["env"]["CLAUDE_CODE_DISABLE_ARTIFACT"], "0");
    storage::restore(&paths, Client::Claude).unwrap();
    assert_eq!(
        fs::read_to_string(paths.target(FileKind::ClaudeSettings)).unwrap(),
        original
    );
}

#[test]
fn vscode_config_directory_controls_import_preview_and_restore_target() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(home.path());
    let custom = home.path().join("claude-special");
    let original_settings = format!(
        r#"{{"claudeCode.environmentVariables":[{{"name":"CLAUDE_CONFIG_DIR","value":"{}"}}]}}"#,
        custom.display()
    );
    storage::atomic_write(
        &paths.target(FileKind::ClaudeVscode),
        original_settings.as_bytes(),
    )
    .unwrap();
    let original_claude = r#"{"hooks":{"Stop":[]}}"#;
    storage::atomic_write(&custom.join("settings.json"), original_claude.as_bytes()).unwrap();
    adapter::apply(&paths, input(Client::ClaudeVscode)).unwrap();
    assert!(custom.join("settings.json").is_file());
    assert!(!paths.target(FileKind::ClaudeSettings).exists());
    let point = restore::list(&paths)
        .into_iter()
        .find(|point| point.client == Client::ClaudeVscode)
        .unwrap();
    assert!(point.available);
    assert!(point
        .files
        .iter()
        .any(|file| file.path == custom.join("settings.json").to_string_lossy()));
    restore::apply(
        &paths,
        Client::ClaudeVscode,
        point.review_id.as_deref().unwrap(),
        false,
    )
    .unwrap();
    assert_eq!(
        fs::read_to_string(custom.join("settings.json")).unwrap(),
        original_claude
    );
    assert_eq!(
        fs::read_to_string(paths.target(FileKind::ClaudeVscode)).unwrap(),
        original_settings
    );
}
