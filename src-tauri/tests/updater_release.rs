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
    let windows = app
        .updater_builder()
        .target("windows-x86_64")
        .executable_path(&executable)
        .build()
        .unwrap()
        .restore_update(manifest.clone())
        .unwrap()
        .unwrap();
    let installer = fs::read(root.join(format!(
        "release/TokenToAPI-Connect-{version}-Windows-x64-setup.exe"
    )))
    .unwrap();
    windows.verify(&installer).unwrap();
    let mut older = manifest.clone();
    older["version"] = Value::String("0.0.0".into());
    assert!(updater.restore_update(older).unwrap().is_none());
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

// Network check + download + install from the actual previous release bundle.
#[tokio::test]
async fn previous_release_checks_downloads_and_installs_next_signed_version() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        process::Command,
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let config: Value =
        serde_json::from_slice(&fs::read(root.join("src-tauri/tauri.conf.json")).unwrap()).unwrap();
    let mut context = mock_context(noop_assets());
    context.package_info_mut().version = "0.2.1".parse().unwrap();
    let previous_config = Command::new("git")
        .args(["show", "v0.2.1:src-tauri/tauri.conf.json"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(previous_config.status.success());
    let previous_config: Value = serde_json::from_slice(&previous_config.stdout).unwrap();
    assert_eq!(
        previous_config["plugins"]["updater"]["pubkey"],
        config["plugins"]["updater"]["pubkey"]
    );
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
    assert!(Command::new("/usr/bin/tar")
        .args(["-xzf"])
        .arg(root.join("release/TokenToAPI-Connect-0.2.1-macOS-arm64.app.tar.gz"))
        .arg("-C")
        .arg(temporary.path())
        .status()
        .unwrap()
        .success());
    let bundle = temporary.path().join("TokenToAPI Connect.app");
    let executable = bundle.join("Contents/MacOS/tokentoapi-connect");
    let old = fs::read(&executable).unwrap();
    let preserved = temporary.path().join("original-config-backup");
    fs::write(&preserved, b"preserve my config").unwrap();
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(root.join("release/latest.json")).unwrap()).unwrap();
    let version = manifest["version"].as_str().unwrap().to_owned();
    let archive = fs::read(root.join(format!(
        "release/TokenToAPI-Connect-{version}-macOS-arm64.app.tar.gz"
    )))
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    manifest["platforms"]["darwin-aarch64"]["url"] =
        Value::String(format!("{endpoint}/app.tar.gz"));
    let body = serde_json::to_vec(&manifest).unwrap();
    let server = std::thread::spawn(move || {
        for (path, data, content_type) in [
            ("/latest.json", body, "application/json"),
            ("/app.tar.gz", archive, "application/octet-stream"),
        ] {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(20)))
                .unwrap();
            let mut bytes = [0; 8192];
            let read = socket.read(&mut bytes).unwrap();
            assert!(String::from_utf8_lossy(&bytes[..read]).starts_with(&format!("GET {path} ")));
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", data.len()).unwrap();
            socket.write_all(&data).unwrap();
        }
    });
    let updater = app
        .updater_builder()
        .executable_path(&executable)
        .endpoints(vec![format!("{endpoint}/latest.json").parse().unwrap()])
        .unwrap()
        .build()
        .unwrap();
    let update = updater
        .check()
        .await
        .unwrap()
        .expect("v0.2.1 must see the new release");
    assert_eq!(update.current_version, "0.2.1");
    assert_eq!(update.version, version);
    let mut downloaded = 0;
    let bytes = update
        .download(|chunk, _| downloaded += chunk, || {})
        .await
        .unwrap();
    assert_eq!(downloaded, bytes.len());
    update.install(bytes).unwrap();
    server.join().unwrap();
    assert_ne!(fs::read(&executable).unwrap(), old);
    assert_eq!(fs::read(preserved).unwrap(), b"preserve my config");
    assert!(Command::new("/usr/bin/codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(&bundle)
        .status()
        .unwrap()
        .success());
    let actual = Command::new("/usr/libexec/PlistBuddy")
        .args(["-c", "Print :CFBundleShortVersionString"])
        .arg(bundle.join("Contents/Info.plist"))
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(actual.stdout).unwrap().trim(), version);
}

#[tokio::test]
#[ignore = "run after publishing the release to verify the public updater endpoint"]
async fn published_release_is_discoverable_and_signed() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let config: Value =
        serde_json::from_slice(&fs::read(root.join("src-tauri/tauri.conf.json")).unwrap()).unwrap();
    let mut context = mock_context(noop_assets());
    context.package_info_mut().version = "0.2.1".parse().unwrap();
    context
        .config_mut()
        .plugins
        .0
        .insert("updater".into(), config["plugins"]["updater"].clone());
    let app = mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)
        .unwrap();
    let updater = app.updater_builder().build().unwrap();
    let update = updater
        .check()
        .await
        .unwrap()
        .expect("public update must be offered to v0.2.1");
    assert_eq!(update.version, env!("CARGO_PKG_VERSION"));
    let bytes = update.download(|_, _| {}, || {}).await.unwrap();
    update.verify(&bytes).unwrap();
}

#[test]
fn previous_release_checks_downloads_and_upgrades_an_isolated_real_app() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        process::Command,
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let previous = std::env::var("TOKENTOAPI_PREVIOUS_MAC_ZIP")
        .expect("Set the previous stable macOS ZIP path for release acceptance");
    let temporary = tempfile::tempdir().unwrap();
    assert!(Command::new("unzip")
        .args(["-q", &previous, "-d"])
        .arg(temporary.path())
        .status()
        .unwrap()
        .success());
    let bundle = temporary
        .path()
        .join("TokenToAPI Connect/TokenToAPI Connect.app");
    let executable = bundle.join("Contents/MacOS/tokentoapi-connect");
    assert!(executable.is_file());
    let old_version = Command::new("/usr/libexec/PlistBuddy")
        .args(["-c", "Print :CFBundleShortVersionString"])
        .arg(bundle.join("Contents/Info.plist"))
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8(old_version.stdout).unwrap().trim(),
        "0.2.1"
    );
    let sentinel = temporary.path().join("configuration-backup-sentinel");
    fs::write(&sentinel, b"keep original configuration").unwrap();
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(root.join("release/latest.json")).unwrap()).unwrap();
    let version = manifest["version"].as_str().unwrap().to_string();
    let archive = fs::read(root.join(format!(
        "release/TokenToAPI-Connect-{version}-macOS-arm64.app.tar.gz"
    )))
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    manifest["platforms"]["darwin-aarch64"]["url"] = Value::String(format!("{endpoint}/update"));
    let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
    let server = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .unwrap();
            let mut request = Vec::new();
            let mut buf = [0; 4096];
            while !request.windows(4).any(|b| b == b"\r\n\r\n") {
                let n = stream.read(&mut buf).unwrap();
                assert!(n > 0);
                request.extend_from_slice(&buf[..n]);
            }
            let body = if request.starts_with(b"GET /latest.json ") {
                &manifest_bytes
            } else {
                assert!(request.starts_with(b"GET /update "));
                &archive
            };
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).unwrap();
            stream.write_all(body).unwrap();
        }
    });
    let config: Value =
        serde_json::from_slice(&fs::read(root.join("src-tauri/tauri.conf.json")).unwrap()).unwrap();
    let mut context = mock_context(noop_assets());
    context.package_info_mut().version = "0.2.1".parse().unwrap();
    let mut updater_config = config["plugins"]["updater"].clone();
    updater_config["dangerousInsecureTransportProtocol"] = Value::Bool(true);
    context
        .config_mut()
        .plugins
        .0
        .insert("updater".into(), updater_config);
    let app = mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)
        .unwrap();
    let updater = app
        .updater_builder()
        .executable_path(&executable)
        .endpoints(vec![format!("{endpoint}/latest.json").parse().unwrap()])
        .unwrap()
        .build()
        .unwrap();
    tauri::async_runtime::block_on(async {
        let update = updater
            .check()
            .await
            .unwrap()
            .expect("previous stable is offered the candidate");
        assert_eq!(update.version, version);
        update.download_and_install(|_, _| {}, || {}).await.unwrap();
    });
    server.join().unwrap();
    assert_eq!(fs::read(sentinel).unwrap(), b"keep original configuration");
    assert!(Command::new("codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(&bundle)
        .status()
        .unwrap()
        .success());
    let installed = Command::new("/usr/libexec/PlistBuddy")
        .args(["-c", "Print :CFBundleShortVersionString"])
        .arg(bundle.join("Contents/Info.plist"))
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(installed.stdout).unwrap().trim(), version);
}
