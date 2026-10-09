//! Restore previews expose file names and timestamps, never saved credentials.
use crate::{
    models::Client,
    storage::{self, Backup, FileKind, Paths},
};
use serde::Serialize;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub const CLIENTS: [Client; 4] = [
    Client::ClaudeDesktop,
    Client::Claude,
    Client::ClaudeVscode,
    Client::Codex,
];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreFile {
    pub label: &'static str,
    pub path: String,
    pub existed_before: bool,
    pub modified_since_import: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePoint {
    pub client: Client,
    pub available: bool,
    pub pending: bool,
    pub created_at: Option<u64>,
    pub files: Vec<RestoreFile>,
    pub review_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub client: Client,
    pub restored_files: usize,
    pub safety_backup_path: String,
    pub was_pending: bool,
}

struct Plan {
    record_path: PathBuf,
    record_text: String,
    backup: Backup,
    current: Vec<Option<String>>,
    point: RestorePoint,
}

fn label(kind: FileKind) -> &'static str {
    match kind {
        FileKind::ClaudeSettings => "Claude Code API, models and original settings",
        FileKind::ClaudeDesktopConfig => "Claude Desktop original mode and preferences",
        FileKind::ClaudeThreepConfig => "Claude third-party mode settings",
        FileKind::ClaudeDesktopProfile => "Claude Desktop gateway, key and models",
        FileKind::ClaudeDesktopMeta => "Claude Desktop enabled gateway",
        FileKind::ClaudeVscode => "VS Code original user settings",
        FileKind::CodexConfig => "Codex original provider, models and settings",
        FileKind::CodexAuth => "Codex original login or API credentials",
        FileKind::CodexCatalog => "Codex model catalog",
    }
}

fn plan(paths: &Paths, client: Client) -> Result<Plan, String> {
    let client = client.backup_client();
    let pending = paths.pending(client);
    let record_path = paths.record(client, pending);
    let (record_text, backup) = storage::read_backup(paths, client, &record_path)?;
    let mut current = vec![];
    let mut files = vec![];
    for entry in &backup.entries {
        let content = storage::read_optional(&paths.target(entry.kind))?;
        let modified = content != entry.before
            && content.as_deref().map(storage::digest).as_deref() != Some(&entry.after_hash);
        files.push(RestoreFile {
            label: label(entry.kind),
            path: entry.path.to_string_lossy().into_owned(),
            existed_before: entry.before.is_some(),
            modified_since_import: modified,
        });
        current.push(content);
    }
    // Bind the confirmation to both the backup and the current file bytes. If a
    // client/editor writes again while the user reviews, require a fresh preview.
    let fingerprint = serde_json::to_string(&(
        storage::digest(&record_text),
        current
            .iter()
            .map(|v| v.as_deref().map(storage::digest))
            .collect::<Vec<_>>(),
    ))
    .map_err(|_| "Could not generate the restore preview.")?;
    let point = RestorePoint {
        client,
        available: true,
        pending,
        created_at: backup
            .created_at
            .or_else(|| storage::record_time(&record_path)),
        files,
        review_id: Some(storage::digest(&fingerprint)),
        error: None,
    };
    Ok(Plan {
        record_path,
        record_text,
        backup,
        current,
        point,
    })
}

pub fn list(paths: &Paths) -> Vec<RestorePoint> {
    CLIENTS
        .into_iter()
        .map(|client| {
            if !paths.has_backup(client) {
                return RestorePoint {
                    client,
                    available: false,
                    pending: false,
                    created_at: None,
                    files: vec![],
                    review_id: None,
                    error: None,
                };
            }
            match plan(paths, client) {
                Ok(plan) => plan.point,
                Err(error) => RestorePoint {
                    client,
                    available: false,
                    pending: paths.pending(client),
                    created_at: None,
                    files: vec![],
                    review_id: None,
                    error: Some(error),
                },
            }
        })
        .collect()
}

#[derive(Serialize)]
struct SafetyFile<'a> {
    kind: FileKind,
    path: &'a std::path::Path,
    contents: &'a Option<String>,
}

pub fn apply(
    paths: &Paths,
    client: Client,
    review_id: &str,
    overwrite_modified: bool,
) -> Result<RestoreResult, String> {
    let _lock = paths.lock()?;
    let plan = plan(paths, client)?;
    if plan.point.review_id.as_deref() != Some(review_id) {
        return Err("The backup or current configuration has changed; refresh the preview before restoring.".into());
    }
    if !overwrite_modified
        && plan
            .point
            .files
            .iter()
            .any(|file| file.modified_since_import)
    {
        return Err("The configuration has new changes after the import. Confirm a backup of the current configuration before restoring.".into());
    }
    let files: Vec<_> = plan
        .backup
        .entries
        .iter()
        .zip(&plan.current)
        .map(|(entry, contents)| SafetyFile {
            kind: entry.kind,
            path: &entry.path,
            contents,
        })
        .collect();
    let snapshot = serde_json::to_vec(&serde_json::json!({
        "version":1, "client":plan.point.client, "createdAt":storage::timestamp(), "files":files
    }))
    .map_err(|_| "Could not back up the current configuration; the restore was not performed.")?;
    if snapshot.len() > 8 * 1024 * 1024 {
        return Err("The current configuration backup exceeds the size limit; the restore was not performed.".into());
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let snapshot_path = paths
        .state
        .join("recovery")
        .join(format!("{}-{nonce}.json", plan.point.client.id()));
    storage::atomic_write(&snapshot_path, &snapshot)?;

    for (entry, expected) in plan.backup.entries.iter().zip(&plan.current) {
        if storage::read_optional(&entry.path)? != *expected {
            return Err("The current configuration changed again; close the related client and refresh the preview.".into());
        }
    }
    let result = (|| {
        for (entry, expected) in plan.backup.entries.iter().zip(&plan.current).rev() {
            if storage::read_optional(&entry.path)? != *expected {
                return Err(
                    "Another program wrote during the restore; the operation was stopped."
                        .to_string(),
                );
            }
            if *expected == entry.before {
                continue;
            }
            match &entry.before {
                Some(original) => storage::atomic_write(&entry.path, original.as_bytes())?,
                None => fs::remove_file(&entry.path).map_err(|_| {
                    "Could not undo the newly created configuration file.".to_string()
                })?,
            }
        }
        for entry in &plan.backup.entries {
            if storage::read_optional(&entry.path)? != entry.before {
                return Err(
                    "Post-restore configuration verification failed; close the client and retry."
                        .into(),
                );
            }
        }
        if storage::read_optional(&plan.record_path)?.as_deref() != Some(&plan.record_text) {
            return Err("The restore record has changed; refresh the status.".into());
        }
        fs::remove_file(&plan.record_path)
            .map_err(|_| "The original configuration was written back, but the restore record was not cleaned up; refresh and retry.".to_string())?;
        Ok(())
    })();
    if let Err(error) = result {
        return Err(format!(
            "{error} The pre-restore configuration has been kept in {}.",
            snapshot_path.display()
        ));
    }
    Ok(RestoreResult {
        client: plan.point.client,
        restored_files: plan.backup.entries.len(),
        safety_backup_path: snapshot_path.to_string_lossy().into_owned(),
        was_pending: plan.point.pending,
    })
}
