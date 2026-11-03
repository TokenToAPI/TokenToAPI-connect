use serde_json::json;
use std::fs;
use tokentoapi_connect_lib::{
    adapter,
    models::{self, Client, DiscoveredModels, ModelChoice},
    restore,
    storage::{self, FileKind, Paths},
};
fn input(model: &str) -> models::ValidatedInput {
    let data = DiscoveredModels {
        client: Client::Gemini,
        api_key: "fixture-only-key".into(),
        root: "https://gateway.example".into(),
        payload: json!({}),
        choices: vec![ModelChoice {
            id: model.into(),
            name: model.into(),
        }],
    };
    models::prepare_selection(&data, &[model.into()], model).unwrap()
}
#[test]
fn gemini_reimport_preserves_settings_and_restores_both_files_exactly() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(temp.path());
    let settings = "{\"mcpServers\":{\"personal\":{\"command\":\"my-server\"}},\"security\":{\"auth\":{\"selectedType\":\"oauth-personal\"}},\"ui\":{\"theme\":\"dark\"}}\n";
    let env = "# Personal variables\nOTHER='keep me'\nexport GEMINI_API_KEY=old\nGOOGLE_GEMINI_BASE_URL=https://old.example\n";
    storage::atomic_write(&paths.target(FileKind::GeminiSettings), settings.as_bytes()).unwrap();
    storage::atomic_write(&paths.target(FileKind::GeminiEnv), env.as_bytes()).unwrap();
    adapter::apply(&paths, input("gemini-2.5-pro")).unwrap();
    let doc: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(paths.target(FileKind::GeminiSettings)).unwrap())
            .unwrap();
    assert_eq!(doc["mcpServers"]["personal"]["command"], "my-server");
    assert_eq!(doc["ui"]["theme"], "dark");
    assert_eq!(doc["security"]["auth"]["selectedType"], "gemini-api-key");
    assert!(
        !adapter::apply(&paths, input("gemini-2.5-pro"))
            .unwrap()
            .changed
    );
    adapter::apply(&paths, input("gemini-2.5-flash")).unwrap();
    let text = fs::read_to_string(paths.target(FileKind::GeminiEnv)).unwrap();
    assert!(text.contains("OTHER='keep me'"));
    assert!(!text.contains("old.example"));
    assert!(text.contains("GEMINI_MODEL='gemini-2.5-flash'"));
    let point = restore::list(&paths)
        .into_iter()
        .find(|p| p.client == Client::Gemini)
        .unwrap();
    assert_eq!(point.files.len(), 2);
    restore::apply(
        &paths,
        Client::Gemini,
        point.review_id.as_deref().unwrap(),
        false,
    )
    .unwrap();
    assert_eq!(
        fs::read_to_string(paths.target(FileKind::GeminiSettings)).unwrap(),
        settings
    );
    assert_eq!(
        fs::read_to_string(paths.target(FileKind::GeminiEnv)).unwrap(),
        env
    );
    assert!(!paths.target(FileKind::CodexConfig).exists());
}
#[test]
fn invalid_settings_or_multiline_environment_never_partially_write() {
    for (settings, env) in [("{broken", "OTHER=ok\n"), ("{}", "OTHER='multi\nline'\n")] {
        let temp = tempfile::tempdir().unwrap();
        let paths = Paths::for_home(temp.path());
        storage::atomic_write(&paths.target(FileKind::GeminiSettings), settings.as_bytes())
            .unwrap();
        storage::atomic_write(&paths.target(FileKind::GeminiEnv), env.as_bytes()).unwrap();
        assert!(adapter::apply(&paths, input("gemini-2.5-pro")).is_err());
        assert_eq!(
            fs::read_to_string(paths.target(FileKind::GeminiSettings)).unwrap(),
            settings
        );
        assert_eq!(
            fs::read_to_string(paths.target(FileKind::GeminiEnv)).unwrap(),
            env
        );
        assert!(!paths.has_backup(Client::Gemini));
    }
}
#[test]
fn fresh_install_restore_removes_generated_files_and_keeps_oauth() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(temp.path());
    storage::atomic_write(&paths.gemini.join("oauth_creds.json"), b"sentinel").unwrap();
    adapter::apply(&paths, input("gemini-2.5-pro")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(paths.target(FileKind::GeminiEnv))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    let point = restore::list(&paths)
        .into_iter()
        .find(|p| p.client == Client::Gemini)
        .unwrap();
    restore::apply(
        &paths,
        Client::Gemini,
        point.review_id.as_deref().unwrap(),
        false,
    )
    .unwrap();
    assert!(!paths.target(FileKind::GeminiEnv).exists());
    assert!(!paths.target(FileKind::GeminiSettings).exists());
    assert_eq!(
        fs::read(paths.gemini.join("oauth_creds.json")).unwrap(),
        b"sentinel"
    );
}
#[test]
fn native_catalog_names_and_non_chat_models_are_handled() {
    let payload = json!({"models":[{"name":"models/gemini-2.5-pro"},{"name":"models/gemini-2.5-flash-image"},{"name":"models/embedding-001"}]});
    assert!(models::catalog_models(&payload).contains(&"gemini-2.5-pro".into()));
    for model in [
        "gemini-2.5-flash-image",
        "gemini-2.5-flash-preview-tts",
        "gpt-5",
        "embedding-001",
    ] {
        assert!(!models::gemini_chat_model(model));
    }
}
