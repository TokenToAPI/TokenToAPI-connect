#![cfg(all(feature = "release-tests", target_os = "macos"))]

use serde_json::Value;
use std::{fs, path::PathBuf};
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri_plugin_updater::UpdaterExt;

// Runs against the actual signed release, replacing only a temporary fake app.
#[test]
fn signed_release_replaces_app_and_rejects_tampering() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let config: Value =
        serde_json::from_slice(&fs::read(root.join("src-tauri/tauri.conf.json")).unwrap()).unwrap();
    let mut context = mock_context(noop_assets());
    context
        .config_mut()
        .plugins
        .0
        .insert("updater".into(), config["plugins"]["updater"].clone());
    let app = mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)
        .unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let bundle = temporary.path().join("TokenToAPI Connect.app");
    let executable = bundle.join("Contents/MacOS/tokentoapi-connect");
    fs::create_dir_all(executable.parent().unwrap()).unwrap();
    fs::write(&executable, b"old executable").unwrap();
    let preserved = temporary.path().join("original-config-backup");
    fs::write(&preserved, b"preserve my config").unwrap();
    let updater = app
        .updater_builder()
        .executable_path(&executable)
        .build()
        .unwrap();
    let manifest: Value =
        serde_json::from_slice(&fs::read(root.join("release/latest.json")).unwrap()).unwrap();
    let version = manifest["version"].as_str().unwrap();
    let bytes = fs::read(root.join(format!(
        "release/TokenToAPI-Connect-{version}-macOS-arm64.app.tar.gz"
    )))
    .unwrap();
    let update = updater.restore_update(manifest.clone()).unwrap().unwrap();
    update.verify(&bytes).unwrap();
    let mut corrupt = bytes.clone();
    let last = corrupt.len() - 1;
    corrupt[last] ^= 1;
    assert!(update.verify(&corrupt).is_err());
    assert!(update.install(&corrupt).is_err());
    assert_eq!(fs::read(&executable).unwrap(), b"old executable");
    let mut substituted = manifest;
    substituted["version"] = Value::String("99.0.0".into());
    assert!(updater
        .restore_update(substituted)
        .unwrap()
        .unwrap()
        .verify(&bytes)
        .is_err());
    update.install(&bytes).unwrap();
    assert!(fs::metadata(&executable).unwrap().len() > 1_000_000);
    assert!(bundle.join("Contents/Info.plist").exists());
    assert_eq!(fs::read(preserved).unwrap(), b"preserve my config");
}
