use std::{
    io::{Read, Write},
    net::TcpListener,
};
use tokentoapi_connect_lib::handoff;

#[tokio::test]
async fn redeemed_handoff_imports_efforts_and_restores_both_codex_clients() {
    use tokentoapi_connect_lib::{adapter, models, restore, storage};
    for client in [models::Client::Codex, models::Client::CodexDesktop] {
        let body = r#"{"code":0,"data":{"version":1,"providerName":"TokenToAPI","baseUrl":"https://example.test/v1","apiKey":"sk-local-fixture-only","suggestedClient":"codex","allowedClients":["codex","codex-desktop"]}}"#;
        let (root, request) = server("200 OK", "", body);
        let link = format!(
            "tokentoapi-connect://v1/import?issuer={root}&code={}",
            "A".repeat(43)
        );
        assert!(!link.contains("sk-local-fixture-only"));
        let setup = handoff::redeem(&handoff::parse(&link, true).unwrap(), true)
            .await
            .unwrap();
        assert!(!request.join().unwrap().contains("sk-local-fixture-only"));
        let home = tempfile::tempdir().unwrap();
        let paths = storage::Paths::for_home(home.path());
        let original = b"# original client settings\nmodel = \"original\"\nmodel_reasoning_effort = \"ultra\"\n";
        storage::atomic_write(&paths.target(storage::FileKind::CodexConfig), original).unwrap();
        let selected = vec!["gpt-6-astra".to_string(), "gpt-5.6-sol".to_string()];
        let catalog = models::build_codex_catalog(
            &serde_json::json!({"data":[{"id":"gpt-6-astra"},{"id":"gpt-5.6-sol"}]}),
            &selected,
        )
        .unwrap();
        let imported = models::ValidatedInput {
            client,
            api_key: setup.api_key,
            root: setup.base_url,
            model: selected[0].clone(),
            selected_models: selected,
            catalog: Some(catalog),
            options: Default::default(),
        };
        assert!(adapter::apply(&paths, imported).unwrap().changed);
        let config = std::fs::read_to_string(paths.target(storage::FileKind::CodexConfig)).unwrap();
        assert!(config.contains("model_reasoning_effort = \"ultra\""));
        assert!(config.contains("https://example.test/v1"));
        let saved: serde_json::Value = serde_json::from_slice(
            &std::fs::read(paths.target(storage::FileKind::CodexCatalog)).unwrap(),
        )
        .unwrap();
        for model in saved["models"].as_array().unwrap() {
            assert!(model["supported_reasoning_levels"]
                .as_array()
                .unwrap()
                .iter()
                .any(|level| level["effort"] == "ultra"));
        }
        let point = restore::list(&paths)
            .into_iter()
            .find(|point| point.client == client.backup_client())
            .unwrap();
        restore::apply(&paths, client, point.review_id.as_deref().unwrap(), false).unwrap();
        assert_eq!(
            std::fs::read(paths.target(storage::FileKind::CodexConfig)).unwrap(),
            original
        );
        assert!(!paths.target(storage::FileKind::CodexCatalog).exists());
    }
}

#[test]
fn strict_handoff_links() {
    let good = format!(
        "tokentoapi-connect://v1/import?issuer=https%3A%2F%2Fexample.test&code={}",
        "A".repeat(43)
    );
    let parsed = handoff::parse(&good, false).unwrap();
    assert_eq!(parsed.issuer, "https://example.test");
    for bad in [
        good.replace("//v1/", "//v2/"),
        good.replace("/import?", "/other?"),
        format!("{good}&code={}", "A".repeat(43)),
        format!("{good}&apiKey=secret"),
        good.replace("https%3A", "http%3A"),
        good.replace("example.test", "user%3Asecret%40example.test"),
        good.replace("example.test", "example.test%2Fpath"),
        good.replace(&"A".repeat(43), "short"),
    ] {
        assert!(handoff::parse(&bad, false).is_err());
    }
}
fn server(status: &str, extra: &str, body: &str) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let response=format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
    let thread = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut bytes = Vec::new();
        let mut buf = [0; 4096];
        loop {
            let n = socket.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            bytes.extend_from_slice(&buf[..n]);
            let text = String::from_utf8_lossy(&bytes);
            if let Some((head, tail)) = text.split_once("\r\n\r\n") {
                let length = head
                    .lines()
                    .find_map(|line| {
                        line.to_lowercase()
                            .strip_prefix("content-length: ")
                            .and_then(|v| v.parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if tail.len() >= length {
                    break;
                }
            }
        }
        socket.write_all(response.as_bytes()).unwrap();
        String::from_utf8_lossy(&bytes).into_owned()
    });
    (url, thread)
}
#[tokio::test]
async fn redeem_uses_body_and_keeps_credentials_native() {
    let body = r#"{"code":0,"data":{"version":1,"providerName":"TokenToAPI","baseUrl":"https://example.test/v1","apiKey":"sk-local-fixture-only","suggestedClient":"codex","allowedClients":["codex","codex-desktop"]}}"#;
    let (root, thread) = server("200 OK", "", body);
    let url = format!(
        "tokentoapi-connect://v1/import?issuer={}&code={}",
        root,
        "A".repeat(43)
    );
    assert!(handoff::parse(&url, false).is_err());
    let result = handoff::redeem(&handoff::parse(&url, true).unwrap(), true)
        .await
        .unwrap();
    assert_eq!(result.base_url, "https://example.test");
    assert_eq!(result.api_key, "sk-local-fixture-only");
    let request = thread.join().unwrap();
    assert!(request.starts_with("POST /api/v1/connect/handoff/redeem HTTP/1.1"));
    assert!(request
        .to_ascii_lowercase()
        .contains("x-tokentoapi-connect-capabilities: grok-cli"));
    assert!(request
        .split_once("\r\n\r\n")
        .unwrap()
        .1
        .contains(&"A".repeat(43)));
}
#[tokio::test]
async fn rejects_redirects_bad_schema_and_redacts_errors() {
    for (status, extra, body) in [
        (
            "302 Found",
            "Location: https://elsewhere.test\r\n",
            "secret",
        ),
        ("400 Bad Request", "", "sk-secret"),
        (
            "200 OK",
            "",
            r#"{"code":0,"data":{"version":99,"apiKey":"sk-secret"}}"#,
        ),
        ("200 OK", "", "invalid sk-secret"),
    ] {
        let (root, thread) = server(status, extra, body);
        let link = handoff::parse(
            &format!(
                "tokentoapi-connect://v1/import?issuer={root}&code={}",
                "A".repeat(43)
            ),
            true,
        )
        .unwrap();
        let error = handoff::redeem(&link, true).await.err().unwrap();
        assert!(!error.contains("sk-secret"));
        assert!(!error.contains(&"A".repeat(43)));
        thread.join().unwrap();
    }
}
#[test]
fn pending_handoffs_deduplicate_and_preserve_accepted_setup() {
    let url = format!(
        "tokentoapi-connect://v1/import?issuer=https%3A%2F%2Fexample.test&code={}",
        "A".repeat(43)
    );
    let mut state = handoff::HandoffState::default();
    assert!(state.receive(&url, false));
    let first = state.pending().unwrap();
    assert!(!state.receive(&url, false));
    assert_eq!(state.pending().unwrap().request_id, first.request_id);
    assert!(state.link("obsolete").is_err());
    state.cancel(&first.request_id);
    assert!(state.pending().is_none());
}

#[test]
fn cancelling_a_queued_link_keeps_the_accepted_credentials_and_review() {
    let first_url = format!(
        "tokentoapi-connect://v1/import?issuer=https%3A%2F%2Fexample.test&code={}",
        "A".repeat(43)
    );
    let mut state = handoff::HandoffState::default();
    assert!(state.receive(&first_url, false));
    let first = state.pending().unwrap();
    let setup: handoff::HandoffSetup = serde_json::from_str(r#"{"version":1,"providerName":"TokenToAPI","baseUrl":"https://example.test","apiKey":"sk-local-fixture-only","suggestedClient":"codex","allowedClients":["codex","codex-desktop"]}"#).unwrap();
    let public = state.accept(&first.request_id, setup).unwrap();
    assert_eq!(
        state.completion_provider("https://example.test"),
        Some("TokenToAPI".to_string())
    );
    assert_eq!(state.completion_provider("https://different.test"), None);
    assert!(!serde_json::to_string(&public)
        .unwrap()
        .contains("sk-local-fixture-only"));
    let second_url = first_url.replace(&"A".repeat(43), &("A".repeat(42) + "E"));
    assert!(state.receive(&second_url, false));
    let second = state.pending().unwrap();
    assert!(!state.cancel(&second.request_id));
    let input = state
        .discovery_input(
            &first.request_id,
            tokentoapi_connect_lib::models::Client::CodexDesktop,
        )
        .unwrap();
    assert_eq!(input.api_key, "sk-local-fixture-only");
    assert!(state
        .discovery_input(
            &first.request_id,
            tokentoapi_connect_lib::models::Client::Claude
        )
        .is_err());
    assert!(state.cancel(&first.request_id));
    assert!(state
        .discovery_input(
            &first.request_id,
            tokentoapi_connect_lib::models::Client::Codex
        )
        .is_err());
}

#[test]
fn authoritative_client_choices_support_mixed_and_code_only_groups() {
    use tokentoapi_connect_lib::models::Client;
    for (allowed, suggested, permitted, blocked) in [
        (
            r#"["codex","claude"]"#,
            "codex",
            Client::Claude,
            Client::ClaudeDesktop,
        ),
        (
            r#"["claude","claude-vscode"]"#,
            "claude",
            Client::ClaudeVscode,
            Client::ClaudeDesktop,
        ),
    ] {
        let mut state = handoff::HandoffState::default();
        let url = format!(
            "tokentoapi-connect://v1/import?issuer=https://example.test&code={}",
            "A".repeat(43)
        );
        assert!(state.receive(&url, false));
        let pending = state.pending().unwrap();
        let json = format!(
            r#"{{"version":1,"providerName":"Fixture","baseUrl":"https://example.test","apiKey":"sk-local-fixture-only","suggestedClient":"{suggested}","allowedClients":{allowed}}}"#
        );
        let setup: handoff::HandoffSetup = serde_json::from_str(&json).unwrap();
        state.accept(&pending.request_id, setup).unwrap();
        assert!(state
            .discovery_input(&pending.request_id, permitted)
            .is_ok());
        assert!(state.discovery_input(&pending.request_id, blocked).is_err());
    }
}
