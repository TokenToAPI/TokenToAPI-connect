use crate::{models::Client, storage::Paths};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Compatibility {
    pub installed: bool,
    pub version: Option<String>,
    pub warnings: Vec<String>,
    pub blocked: Option<String>,
}

fn output(program: &Path, args: &[&str]) -> Option<String> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Version probes (including npm .cmd shims) must not flash a console window.
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut child = command.spawn().ok()?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                let out = child.wait_with_output().ok()?;
                if out.stdout.len() > 8192 {
                    return None;
                }
                return String::from_utf8(out.stdout)
                    .ok()
                    .map(|v| v.trim().to_string());
            }
            Ok(None) if start.elapsed() < Duration::from_secs(2) => {
                std::thread::sleep(Duration::from_millis(20))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

fn executable(paths: &Paths, name: &str) -> Option<PathBuf> {
    let mut roots: Vec<_> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    roots.push(paths.home.join(".local/bin"));
    #[cfg(not(windows))]
    roots.extend([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ]);
    #[cfg(windows)]
    {
        roots.push(
            std::env::var_os("APPDATA")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .unwrap_or_else(|| paths.home.join("AppData/Roaming"))
                .join("npm"),
        );
        roots.push(
            std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .unwrap_or_else(|| paths.home.join("AppData/Local"))
                .join("Microsoft/WinGet/Links"),
        );
    }
    find_executable(&roots, name, cfg!(windows))
}

fn find_executable(roots: &[PathBuf], name: &str, windows: bool) -> Option<PathBuf> {
    // npm also creates extensionless POSIX scripts on Windows; those cannot be launched there.
    let extensions: &[&str] = if windows {
        &["exe", "com", "cmd", "bat"]
    } else {
        &[""]
    };
    roots
        .iter()
        .filter(|p| p.is_absolute())
        .flat_map(|p| {
            extensions.iter().map(move |ext| {
                if ext.is_empty() {
                    p.join(name)
                } else {
                    p.join(format!("{name}.{ext}"))
                }
            })
        })
        .find(|p| p.is_file())
}

#[cfg(windows)]
fn windows_managed_provider() -> bool {
    use winreg::{enums::*, RegKey};
    // Claude gives machine policy precedence over user policy, even for app-only settings.
    for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        if let Ok(key) = RegKey::predef(hive)
            .open_subkey_with_flags(r"SOFTWARE\Policies\Claude", KEY_READ | KEY_WOW64_64KEY)
        {
            let names: Vec<_> = key
                .enum_values()
                .filter_map(Result::ok)
                .filter(|(_, value)| matches!(value.vtype, REG_SZ | REG_EXPAND_SZ | REG_DWORD))
                .map(|(name, _)| name)
                .collect();
            if !names.is_empty() {
                return names
                    .iter()
                    .any(|name| name.eq_ignore_ascii_case("inferenceProvider"));
            }
        }
    }
    false
}

fn version_number(text: &str) -> Option<String> {
    text.split_whitespace()
        .find(|part| part.chars().next().is_some_and(|c| c.is_ascii_digit()) && part.contains('.'))
        .map(str::to_owned)
}

#[cfg(target_os = "macos")]
fn app(paths: &Paths, names: &[&str]) -> Option<PathBuf> {
    [
        PathBuf::from("/Applications"),
        paths.home.join("Applications"),
    ]
    .into_iter()
    .flat_map(|root| names.iter().map(move |name| root.join(name)))
    .find(|p| {
        if !p.is_dir() {
            return false;
        }
        // Some builds display "ChatGPT" while retaining the Codex bundle ID.
        // A regular ChatGPT-only app must not be reported as installed Codex.
        if p.file_name().and_then(|s| s.to_str()) == Some("ChatGPT.app") {
            return output(
                Path::new("/usr/libexec/PlistBuddy"),
                &[
                    "-c",
                    "Print :CFBundleIdentifier",
                    p.join("Contents/Info.plist").to_str().unwrap_or(""),
                ],
            )
            .as_deref()
                == Some("com.openai.codex");
        }
        true
    })
}

pub fn inspect(paths: &Paths, client: Client) -> Compatibility {
    if paths.sandbox {
        return Compatibility {
            installed: true,
            warnings: vec![
                "Isolated test: the configuration will not be written to your real home directory."
                    .into(),
            ],
            ..Default::default()
        };
    }
    let mut result = Compatibility::default();
    match client {
        Client::Claude | Client::Codex => {
            if let Some(path) = executable(
                paths,
                if client == Client::Claude {
                    "claude"
                } else {
                    "codex"
                },
            ) {
                result.installed = true;
                result.version = output(&path, &["--version"])
                    .as_deref()
                    .and_then(version_number);
            }
            if client == Client::Claude
                && result
                    .version
                    .as_deref()
                    .is_some_and(|v| v.starts_with("1."))
            {
                result.warnings.push("Detected Claude Code 1.x: the gateway and default model can be read, but the selected-models menu is not supported. Upgrade Claude Code to use multi-model selection.".into());
            }
        }
        Client::ClaudeVscode => {
            if let Ok(entries) = fs::read_dir(paths.home.join(".vscode/extensions")) {
                for entry in entries.flatten() {
                    if entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with("anthropic.claude-code-")
                    {
                        result.installed = true;
                        if let Ok(text) = fs::read_to_string(entry.path().join("package.json")) {
                            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                                result.version = value["version"].as_str().map(str::to_owned);
                            }
                        }
                    }
                }
            }
            result.warnings.push("Writes the VS Code default user configuration and syncs Claude Code model settings. Custom VS Code Profiles, Remote SSH and WSL must be configured in their own environments.".into());
        }
        Client::ClaudeDesktop | Client::CodexDesktop => {
            #[cfg(target_os = "macos")]
            if let Some(path) = app(
                paths,
                if client == Client::ClaudeDesktop {
                    &["Claude.app"]
                } else {
                    &["Codex.app", "ChatGPT.app"]
                },
            ) {
                result.installed = true;
                result.version = output(
                    Path::new("/usr/libexec/PlistBuddy"),
                    &[
                        "-c",
                        "Print :CFBundleShortVersionString",
                        path.join("Contents/Info.plist").to_str().unwrap_or(""),
                    ],
                );
            }
            #[cfg(not(target_os = "macos"))]
            {
                result.installed = if client == Client::ClaudeDesktop {
                    paths.claude_desktop.is_dir()
                } else {
                    paths.codex.is_dir()
                };
                result
                    .warnings
                    .push("Configuration directory located; the client version on this system must be confirmed manually.".into());
            }
            if client == Client::ClaudeDesktop {
                #[cfg(windows)]
                if windows_managed_provider() {
                    result.blocked = Some("Claude Desktop’s inference service is managed by your organization; local imports will not take effect. Ask an administrator to adjust the gateway configuration.".into());
                }
                #[cfg(target_os = "macos")]
                {
                    let user = paths
                        .home
                        .file_name()
                        .and_then(|v| v.to_str())
                        .unwrap_or("");
                    for managed in [
                        PathBuf::from(
                            "/Library/Managed Preferences/com.anthropic.claudefordesktop.plist",
                        ),
                        PathBuf::from("/Library/Managed Preferences")
                            .join(user)
                            .join("com.anthropic.claudefordesktop.plist"),
                    ] {
                        if managed.is_file()
                            && output(
                                Path::new("/usr/libexec/PlistBuddy"),
                                &[
                                    "-c",
                                    "Print :inferenceProvider",
                                    managed.to_str().unwrap_or(""),
                                ],
                            )
                            .is_some()
                        {
                            result.blocked = Some("Claude Desktop’s inference service is managed by your organization; local imports will not take effect. Ask an administrator to adjust the gateway configuration.".into());
                        }
                    }
                }
            }
        }
    }
    if !result.installed {
        result.warnings.insert(
            0,
            format!(
                "Could not find {} in the usual locations. You can save the configuration anyway; it only takes effect after installing or opening a version that supports it.",
                client.name()
            ),
        );
    }
    if client.is_codex() {
        result
            .warnings
            .push("Codex Desktop and Terminal share one configuration; importing and restoring affect the local sessions of both.".into());
    }
    result
}

pub fn next_steps(client: Client) -> Vec<String> {
    match client {
        Client::ClaudeDesktop => vec![
            if cfg!(windows) {
                "Quit Claude from the system tray and reopen it; closing the window alone may leave it running in the background.".into()
            } else {
                "Quit Claude completely (⌘Q on macOS), then reopen the desktop app.".into()
            },
            "Enter third-party inference mode; if a deployment picker appears, choose third-party service. Chat, Cowork and local Code use the same gateway.".into(),
            "Open a new conversation, confirm the model menu, then send a test message. Original Claude.ai cloud chats remain in their original mode.".into(),
        ],
        Client::Claude => vec!["Exit the current Claude Code process and run claude again in a new terminal.".into(), "Run /status to confirm the API address; use /model to check the model. Upgrade first if you are on an old version.".into()],
        Client::ClaudeVscode => vec!["Run Developer: Reload Window in VS Code, then start a new Claude Code session.".into(), "Confirm the default VS Code Profile is used; workspace settings may override user settings.".into()],
        Client::CodexDesktop => vec!["Quit the Codex desktop app completely, reopen it, and start a new local session.".into(), "Confirm API Key sign-in, the TokenToAPI Connect service and the selected model; existing sessions or cloud tasks will not migrate automatically.".into()],
        Client::Codex => vec!["Exit the current Codex process and run codex again.".into(), "Run /status to confirm the TokenToAPI Connect service and the selected model. The desktop app also needs a restart to take effect.".into()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_resolves_native_and_npm_launchers_without_picking_posix_shim() {
        let temp = tempfile::tempdir().unwrap();
        let bin = temp.path().join("Unicode User Test/bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("claude"), "#!/bin/sh").unwrap();
        fs::write(bin.join("claude.ps1"), "unused").unwrap();
        let roots = [bin.clone()];
        assert_eq!(find_executable(&roots, "claude", true), None);
        fs::write(bin.join("claude.cmd"), "@echo off").unwrap();
        assert_eq!(
            find_executable(&roots, "claude", true),
            Some(bin.join("claude.cmd"))
        );
        fs::write(bin.join("claude.exe"), "fixture").unwrap();
        assert_eq!(
            find_executable(&roots, "claude", true),
            Some(bin.join("claude.exe"))
        );
        assert_eq!(
            find_executable(&roots, "claude", false),
            Some(bin.join("claude"))
        );
    }

    #[test]
    fn launcher_search_preserves_path_order_and_ignores_relative_roots() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("first");
        let second = temp.path().join("second");
        for dir in [&first, &second] {
            fs::create_dir_all(dir).unwrap();
        }
        fs::write(first.join("codex.cmd"), "fixture").unwrap();
        fs::write(second.join("codex.exe"), "fixture").unwrap();
        assert_eq!(
            find_executable(&[PathBuf::from("."), first.clone(), second], "codex", true),
            Some(first.join("codex.cmd"))
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_can_probe_cmd_with_spaces_and_unicode_in_path() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("Unicode User/npm");
        fs::create_dir_all(&dir).unwrap();
        let script = dir.join("claude.cmd");
        fs::write(&script, "@echo off\r\necho 2.0.0 (Claude Code)\r\n").unwrap();
        assert_eq!(
            output(&script, &["--version"])
                .as_deref()
                .and_then(version_number)
                .as_deref(),
            Some("2.0.0")
        );
    }
}
