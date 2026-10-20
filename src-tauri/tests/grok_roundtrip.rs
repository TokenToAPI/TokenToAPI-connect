use serde_json::json;
use std::fs;
use tokentoapi_connect_lib::{
    adapter,
    models::{self, Client, DiscoveredModels, ModelChoice},
    restore,
    storage::{self, FileKind, Paths},
};

fn input(selected: &[&str]) -> models::ValidatedInput {
    let data = DiscoveredModels {
        client: Client::Grok,
        api_key: "sk-fixture-only".into(),
        root: "https://gateway.example".into(),
        choices: selected
            .iter()
            .map(|id| ModelChoice {
                id: id.to_string(),
                name: id.to_string(),
            })
            .collect(),
        payload: json!({"models": selected.iter().map(|id| json!({"slug":id,"context_window":500000,"input_modalities":["text","image"],"default_reasoning_level":"high","supported_reasoning_levels":[{"effort":"low"},{"effort":"high"}]})).collect::<Vec<_>>()}),
    };
    models::prepare_selection(
        &data,
        &selected.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
        selected[0],
    )
    .unwrap()
}

#[test]
fn grok_merge_reimport_and_restore_preserve_unrelated_settings() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(temp.path());
    let path = paths.target(FileKind::GrokConfig);
    let original = "# My Grok settings\n[permissions]\ndefault = \"ask\"\n[model.personal]\nmodel = \"personal-model\"\n[models]\ndefault = \"personal\"\n[mcp_servers.example]\ncommand = \"my-server\"\n";
    storage::atomic_write(&path, original.as_bytes()).unwrap();
    adapter::apply(&paths, input(&["grok-4.6", "grok-4.7"])).unwrap();
    let text = fs::read_to_string(&path).unwrap();
    let doc = text.parse::<toml_edit::DocumentMut>().unwrap();
    assert!(text.contains("# My Grok settings"));
    assert_eq!(doc["permissions"]["default"].as_str(), Some("ask"));
    assert_eq!(
        doc["mcp_servers"]["example"]["command"].as_str(),
        Some("my-server")
    );
    assert_eq!(doc["models"]["allowed_models"].as_array().unwrap().len(), 2);
    let default = doc["models"]["default"].as_str().unwrap();
    let row = &doc["model"][default];
    assert_eq!(row["model"].as_str(), Some("grok-4.6"));
    assert_eq!(row["api_key"].as_str(), Some("sk-fixture-only"));
    assert_eq!(row["api_backend"].as_str(), Some("responses"));
    assert_eq!(row["context_window"].as_integer(), Some(500000));
    assert_eq!(
        row["reasoning_efforts"].as_array_of_tables().unwrap().len(),
        2
    );
    assert!(
        !adapter::apply(&paths, input(&["grok-4.6", "grok-4.7"]))
            .unwrap()
            .changed
    );
    adapter::apply(&paths, input(&["grok-4.7"])).unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert!(!text.contains("grok-4.6"));
    assert!(text.contains("personal-model"));
    let point = restore::list(&paths)
        .into_iter()
        .find(|p| p.client == Client::Grok)
        .unwrap();
    restore::apply(
        &paths,
        Client::Grok,
        point.review_id.as_deref().unwrap(),
        false,
    )
    .unwrap();
    assert_eq!(fs::read_to_string(path).unwrap(), original);
    assert!(!paths.target(FileKind::CodexConfig).exists());
}

#[test]
fn missing_context_and_malformed_config_make_no_writes() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths::for_home(temp.path());
    let mut missing = input(&["grok-new"]);
    let mut catalog: serde_json::Value =
        serde_json::from_str(missing.catalog.as_deref().unwrap()).unwrap();
    catalog["models"][0]["context_window"] = serde_json::Value::Null;
    catalog["models"][0]["max_context_window"] = serde_json::Value::Null;
    missing.catalog = Some(catalog.to_string());
    assert!(adapter::apply(&paths, missing)
        .unwrap_err()
        .contains("context capacity"));
    assert!(!paths.target(FileKind::GrokConfig).exists());
    storage::atomic_write(&paths.target(FileKind::GrokConfig), b"[broken").unwrap();
    assert!(adapter::apply(&paths, input(&["grok-4.7"])).is_err());
    assert_eq!(
        fs::read_to_string(paths.target(FileKind::GrokConfig)).unwrap(),
        "[broken"
    );
}
