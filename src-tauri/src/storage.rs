use crate::models::Client;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct Paths {
    pub home: PathBuf,
    pub claude: PathBuf,
    pub claude_desktop: PathBuf,
    pub claude_threep: PathBuf,
    pub vscode: PathBuf,
    pub codex: PathBuf,
    pub state: PathBuf,
    pub sandbox: bool,
}
impl Paths {
    pub fn for_home(home: &Path) -> Self {
        #[cfg(target_os = "macos")]
        let app_data = home.join("Library/Application Support");
        #[cfg(target_os = "windows")]
        let app_data = home.join("AppData/Local");
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let app_data = home.join(".config");
        #[cfg(target_os = "windows")]
        let vscode = home.join("AppData/Roaming/Code/User/settings.json");
        #[cfg(not(target_os = "windows"))]
        let vscode = app_data.join("Code/User/settings.json");
        Self {
            home: home.to_path_buf(),
            claude: home.join(".claude"),
            claude_desktop: app_data.join("Claude"),
            claude_threep: app_data.join("Claude-3p"),
            vscode,
            codex: home.join(".codex"),
            state: home.join(".tokentoapi-connect"),
            sandbox: true,
        }
    }
    pub fn discover() -> Result<Self, String> {
        // Explicit isolated home for native smoke tests. Never fall back to real home if invalid.
        if cfg!(debug_assertions) {
            if let Some(home) = std::env::var_os("TOKENTOAPI_CONNECT_TEST_HOME") {
                let home = PathBuf::from(home);
                if !home.is_absolute() {
                    return Err("The test directory must be an absolute path.".into());
                }
                return Ok(Self::for_home(&home));
            }
        }
        let home = dirs::home_dir()
            .ok_or("Could not locate the home directory; configuration was not written.")?;
        let override_dir = |key: &str, fallback: PathBuf| -> Result<PathBuf, String> {
            match std::env::var_os(key) {
                Some(v) if !v.is_empty() => {
                    let p = PathBuf::from(v);
                    if !p.is_absolute() {
                        Err(format!("{key} must be an absolute path."))
                    } else {
                        Ok(p)
                    }
                }
                _ => Ok(fallback),
            }
        };
        let mut paths = Self::for_home(&home);
        paths.claude = override_dir("CLAUDE_CONFIG_DIR", paths.claude)?;
        paths.codex = override_dir("CODEX_HOME", paths.codex)?;
        #[cfg(target_os = "windows")]
        {
            let data = override_dir("LOCALAPPDATA", home.join("AppData/Local"))?;
            paths.claude_desktop = data.join("Claude");
            paths.claude_threep = data.join("Claude-3p");
            paths.vscode = override_dir("APPDATA", home.join("AppData/Roaming"))?
                .join("Code/User/settings.json");
        }
        #[cfg(target_os = "linux")]
        {
            let data = override_dir("XDG_CONFIG_HOME", home.join(".config"))?;
            paths.claude_desktop = data.join("Claude");
            paths.claude_threep = data.join("Claude-3p");
            paths.vscode = data.join("Code/User/settings.json");
        }
        paths.sandbox = false;
        Ok(paths)
    }
    pub fn target(&self, kind: FileKind) -> PathBuf {
        match kind {
            FileKind::ClaudeSettings => self.claude.join("settings.json"),
            FileKind::ClaudeDesktopConfig => self.claude_desktop.join("claude_desktop_config.json"),
            FileKind::ClaudeThreepConfig => self.claude_threep.join("claude_desktop_config.json"),
            FileKind::ClaudeDesktopProfile => self.claude_threep.join(format!(
                "configLibrary/{}.json",
                crate::desktop_config::PROFILE_ID
            )),
            FileKind::ClaudeDesktopMeta => self.claude_threep.join("configLibrary/_meta.json"),
            FileKind::ClaudeVscode => self.vscode.clone(),
            FileKind::CodexConfig => self.codex.join("config.toml"),
            FileKind::CodexAuth => self.codex.join("auth.json"),
            FileKind::CodexCatalog => self.codex.join("tokentoapi-models.json"),
        }
    }
    pub(crate) fn record(&self, client: Client, pending: bool) -> PathBuf {
        self.state.join(format!(
            "{}.{}.json",
            client.backup_client().id(),
            if pending { "pending" } else { "backup" }
        ))
    }
    pub fn has_backup(&self, client: Client) -> bool {
        self.record(client, true).is_file() || self.record(client, false).is_file()
    }
    pub fn pending(&self, client: Client) -> bool {
        self.record(client, true).is_file()
    }
    pub fn lock(&self) -> Result<fs::File, String> {
        private_dir(&self.state)?;
        let path = self.state.join("write.lock");
        reject_symlink(&path)?;
        let mut opts = fs::OpenOptions::new();
        opts.create(true).truncate(false).read(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let file = opts
            .open(&path)
            .map_err(|_| "Could not create the configuration lock.")?;
        file.try_lock_exclusive()
            .map_err(|_| "Another import is already in progress; please try again later.")?;
        Ok(file)
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum FileKind {
    ClaudeSettings,
    ClaudeDesktopConfig,
    ClaudeThreepConfig,
    ClaudeDesktopProfile,
    ClaudeDesktopMeta,
    ClaudeVscode,
    CodexConfig,
    CodexAuth,
    CodexCatalog,
}
impl FileKind {
    pub fn primary(client: Client) -> Self {
        match client {
            Client::Claude => Self::ClaudeSettings,
            Client::ClaudeDesktop => Self::ClaudeDesktopProfile,
            Client::ClaudeVscode => Self::ClaudeVscode,
            Client::Codex | Client::CodexDesktop => Self::CodexConfig,
        }
    }
    fn belongs_to(self, client: Client) -> bool {
        match client {
            Client::Claude => self == Self::ClaudeSettings,
            Client::ClaudeDesktop => matches!(
                self,
                Self::ClaudeDesktopConfig
                    | Self::ClaudeThreepConfig
                    | Self::ClaudeDesktopProfile
                    | Self::ClaudeDesktopMeta
            ),
            Client::ClaudeVscode => matches!(self, Self::ClaudeSettings | Self::ClaudeVscode),
            Client::Codex | Client::CodexDesktop => matches!(
                self,
                Self::CodexConfig | Self::CodexAuth | Self::CodexCatalog
            ),
        }
    }
}

pub struct Change {
    pub kind: FileKind,
    pub before: Option<String>,
    pub after: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Entry {
    pub kind: FileKind,
    pub path: PathBuf,
    pub before: Option<String>,
    pub after_hash: String,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct Backup {
    pub version: u8,
    pub client: Client,
    #[serde(default, rename = "createdAt")]
    pub created_at: Option<u64>,
    pub entries: Vec<Entry>,
}

pub(crate) fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

pub(crate) fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(crate) fn record_time(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|t| t.as_secs())
}

pub fn reject_symlink(path: &Path) -> Result<(), String> {
    if let Ok(meta) = fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            return Err(format!(
                "{} is a symbolic link; the first version will not modify it.",
                path.display()
            ));
        }
    }
    Ok(())
}

pub fn read_optional(path: &Path) -> Result<Option<String>, String> {
    reject_symlink(path)?;
    if let Some(parent) = path.parent() {
        reject_symlink(parent)?;
    }
    match fs::metadata(path) {
        Ok(meta) if meta.len() > 8 * 1024 * 1024 => {
            return Err(format!(
                "{} exceeds the configuration file size limit.",
                path.display()
            ))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(format!("Could not read {}.", path.display())),
        _ => {}
    }
    fs::read_to_string(path).map(Some).map_err(|_| {
        format!(
            "Could not read {}; check file permissions and UTF-8 encoding.",
            path.display()
        )
    })
}

fn private_dir(path: &Path) -> Result<(), String> {
    reject_symlink(path)?;
    if !path.exists() {
        fs::create_dir_all(path).map_err(|_| format!("Could not create {}.", path.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o700))
                .map_err(|_| "Could not set directory permissions.")?;
        }
    }
    Ok(())
}

// Same-directory staged write + atomic replacement, as in CC Switch's config writer.
// tempfile supplies cross-platform replacement and unique, private temporary files.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    reject_symlink(path)?;
    let parent = path.parent().ok_or("Invalid configuration path.")?;
    private_dir(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|_| "Could not create a temporary configuration file.")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temp.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|_| "Could not protect the configuration file permissions.")?;
    }
    temp.write_all(bytes)
        .and_then(|_| temp.as_file().sync_all())
        .map_err(|_| "Failed to write the temporary configuration.")?;
    temp.persist(path).map_err(|_| {
        format!(
            "Could not replace {}; close any program holding the file and retry.",
            path.display()
        )
    })?;
    Ok(())
}

pub fn commit(paths: &Paths, client: Client, changes: Vec<Change>) -> Result<bool, String> {
    let client = client.backup_client();
    if paths.pending(client) {
        return Err("A previous import was interrupted; click Restore Config first.".into());
    }
    let changes: Vec<_> = changes
        .into_iter()
        .filter(|c| c.before.as_deref() != Some(&c.after))
        .collect();
    if changes.is_empty() {
        return Ok(false);
    }
    // Re-check the entire plan before touching any target.
    for c in &changes {
        if read_optional(&paths.target(c.kind))? != c.before {
            return Err("The configuration was just modified by another program; close the client and retry.".into());
        }
    }
    let backup = Backup {
        version: 1,
        client,
        created_at: Some(timestamp()),
        entries: changes
            .iter()
            .map(|c| Entry {
                kind: c.kind,
                path: paths.target(c.kind),
                before: c.before.clone(),
                after_hash: digest(&c.after),
            })
            .collect(),
    };
    // The pending journal always captures this operation, so rollback returns to
    // the last working import. The lasting restore point keeps the first baseline.
    let previous_path = paths.record(client, false);
    let mut baseline = if previous_path.exists() {
        let (_, mut saved) = read_backup(paths, client, &previous_path)?;
        saved.created_at = saved.created_at.or_else(|| record_time(&previous_path));
        saved
    } else {
        Backup {
            version: 1,
            client,
            created_at: backup.created_at,
            entries: vec![],
        }
    };
    for new_entry in &backup.entries {
        if let Some(old_entry) = baseline
            .entries
            .iter_mut()
            .find(|entry| entry.kind == new_entry.kind)
        {
            old_entry.after_hash.clone_from(&new_entry.after_hash);
        } else {
            baseline.entries.push(new_entry.clone());
        }
    }
    let record =
        serde_json::to_vec(&backup).map_err(|_| "Could not generate the configuration backup.")?;
    let baseline_record = serde_json::to_vec(&baseline)
        .map_err(|_| "Could not generate the original configuration backup.")?;
    if record.len() > 8 * 1024 * 1024 || baseline_record.len() > 8 * 1024 * 1024 {
        return Err("The configuration backup exceeds the size limit; client configuration was not modified.".into());
    }
    let pending = paths.record(client, true);
    atomic_write(&pending, &record)?;
    let result = (|| {
        for c in &changes {
            if read_optional(&paths.target(c.kind))? != c.before {
                return Err(
                    "The configuration was modified by another program; the import was stopped."
                        .to_string(),
                );
            }
            atomic_write(&paths.target(c.kind), c.after.as_bytes())?;
        }
        for c in &changes {
            if read_optional(&paths.target(c.kind))?.as_deref() != Some(c.after.as_str()) {
                return Err("Post-write configuration verification failed; restoring the original configuration.".to_string());
            }
        }
        atomic_write(&paths.record(client, false), &baseline_record)?;
        fs::remove_file(&pending)
            .map_err(|_| "The configuration was written, but the completion marker failed; use Restore Config to verify.".to_string())?;
        Ok(())
    })();
    if let Err(error) = result {
        return match restore_record(paths, client, &pending) {
            Ok(()) => Err(format!("{error} The pre-import files have been restored.")),
            Err(_) => Err(format!(
                "{error} The backup has been kept; click Restore Config first."
            )),
        };
    }
    Ok(true)
}

pub(crate) fn read_backup(
    paths: &Paths,
    client: Client,
    record: &Path,
) -> Result<(String, Backup), String> {
    let text = read_optional(record)?.ok_or("No configuration backup to restore.")?;
    let backup: Backup = serde_json::from_str(&text)
        .map_err(|_| "The backup file is corrupted; no configuration was modified.")?;
    if backup.version != 1
        || backup.client != client
        || backup.entries.is_empty()
        || backup.entries.len() > 4
        || backup.entries.iter().any(|e| !e.kind.belongs_to(client))
    {
        return Err(
            "The backup file does not match this tool; no configuration was modified.".into(),
        );
    }
    for (index, e) in backup.entries.iter().enumerate() {
        if backup.entries[..index].iter().any(|old| old.kind == e.kind) {
            return Err(
                "The backup contains duplicate files; no configuration was modified.".into(),
            );
        }
        if e.path != paths.target(e.kind) {
            return Err("The client configuration directory has changed; automatic restore stopped. Switch back to the original directory to restore.".into());
        }
    }
    Ok((text, backup))
}

fn restore_record(paths: &Paths, client: Client, record: &Path) -> Result<(), String> {
    let (_, backup) = read_backup(paths, client, record)?;
    for e in &backup.entries {
        let current = read_optional(&paths.target(e.kind))?;
        if current != e.before && current.as_deref().map(digest).as_deref() != Some(&e.after_hash) {
            return Err("The configuration was modified again after the import; automatic restore stopped so your new edits are kept. The backup is still in the .tokentoapi-connect directory.".into());
        }
    }
    for e in backup.entries.iter().rev() {
        let path = paths.target(e.kind);
        let current = read_optional(&path)?;
        if current == e.before {
            continue;
        }
        if current.as_deref().map(digest).as_deref() != Some(&e.after_hash) {
            return Err("Another program modified the configuration during restore; the operation was stopped.".into());
        }
        match &e.before {
            Some(text) => atomic_write(&path, text.as_bytes())?,
            None => fs::remove_file(&path)
                .map_err(|_| format!("Could not restore {}.", path.display()))?,
        }
    }
    fs::remove_file(record).map_err(|_| {
        "The original configuration was restored, but the backup marker was not cleaned up."
            .to_string()
    })?;
    Ok(())
}

pub fn restore(paths: &Paths, client: Client) -> Result<(), String> {
    let client = client.backup_client();
    let _lock = paths.lock()?;
    let pending = paths.record(client, true);
    let record = if pending.exists() {
        pending
    } else {
        paths.record(client, false)
    };
    restore_record(paths, client, &record)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interrupted_two_file_import_can_be_recovered() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::for_home(dir.path());
        let original = "{\"tokens\":{\"refresh_token\":\"fake-old-token\"}}";
        atomic_write(&paths.target(FileKind::CodexAuth), b"new-auth").unwrap();
        let backup = Backup {
            version: 1,
            client: Client::Codex,
            created_at: None,
            entries: vec![
                Entry {
                    kind: FileKind::CodexAuth,
                    path: paths.target(FileKind::CodexAuth),
                    before: Some(original.into()),
                    after_hash: digest("new-auth"),
                },
                Entry {
                    kind: FileKind::CodexConfig,
                    path: paths.target(FileKind::CodexConfig),
                    before: None,
                    after_hash: digest("new-config"),
                },
            ],
        };
        atomic_write(
            &paths.record(Client::Codex, true),
            &serde_json::to_vec(&backup).unwrap(),
        )
        .unwrap();
        restore(&paths, Client::Codex).unwrap();
        assert_eq!(
            read_optional(&paths.target(FileKind::CodexAuth))
                .unwrap()
                .as_deref(),
            Some(original)
        );
        assert!(!paths.target(FileKind::CodexConfig).exists());
        assert!(!paths.pending(Client::Codex));
    }
    #[test]
    fn concurrent_writer_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::for_home(dir.path());
        let lock = paths.lock().unwrap();
        assert!(paths.lock().is_err());
        drop(lock);
        assert!(paths.lock().is_ok());
    }
    #[test]
    fn stale_plan_does_not_overwrite_the_newer_file() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::for_home(dir.path());
        atomic_write(&paths.target(FileKind::ClaudeSettings), b"newer").unwrap();
        assert!(commit(
            &paths,
            Client::Claude,
            vec![Change {
                kind: FileKind::ClaudeSettings,
                before: Some("old".into()),
                after: "target".into()
            }]
        )
        .is_err());
        assert_eq!(
            read_optional(&paths.target(FileKind::ClaudeSettings))
                .unwrap()
                .as_deref(),
            Some("newer")
        );
        assert!(!paths.has_backup(Client::Claude));
    }
}
