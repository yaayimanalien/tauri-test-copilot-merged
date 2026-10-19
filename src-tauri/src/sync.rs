use crate::{
    crypto::{self, Entry, Header, Payload},
    drive::{DriveClient, FileMetadata},
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ItemState<'a> {
    Unknown,
    Live { updated: i64, stamp: &'a str },
    Dead { deleted_at: i64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    UploadEntry,
    DownloadEntry,
    UploadTombstone,
    AdoptTombstone,
    RemoveLocalEntry,
    DeleteRemoteEntry,
    DeleteRemoteTombstone,
    Unchanged,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub uploaded: usize,
    pub downloaded: usize,
    pub removed: usize,
    pub unchanged: usize,
    pub at: i64,
}

#[derive(Deserialize, Serialize)]
struct Tombstone {
    id: String,
    deleted_at: i64,
}

fn stamp(parts: &[&[u8]]) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update((part.len() as u64).to_le_bytes());
        digest.update(part);
    }
    digest.finalize()[..12]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn entry_stamp(entry: &Entry) -> String {
    let updated = entry.updated_at.to_string();
    stamp(&[
        entry.id.as_bytes(),
        entry.title.as_bytes(),
        entry.body.as_bytes(),
        updated.as_bytes(),
    ])
}

pub fn tombstone_stamp(id: &str, deleted_at: i64) -> String {
    let deleted_at = deleted_at.to_string();
    stamp(&[b"t", id.as_bytes(), deleted_at.as_bytes()])
}

fn properties(stamp: String, updated: i64) -> HashMap<String, String> {
    HashMap::from([
        ("h".to_string(), stamp),
        ("u".to_string(), updated.to_string()),
    ])
}

fn property_time(file: Option<&FileMetadata>) -> i64 {
    file.and_then(|file| file.app_properties.get("u"))
        .and_then(|value| value.parse().ok())
        .unwrap_or_default()
}

fn seal_json<T: Serialize>(key: &[u8; 32], value: &T, aad: &[u8]) -> Result<Vec<u8>, String> {
    let json = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    Ok(B64.encode(crypto::seal(key, &json, aad)?).into_bytes())
}

fn open_json<T: for<'de> Deserialize<'de>>(
    key: &[u8; 32],
    blob: &[u8],
    aad: &[u8],
) -> Result<T, String> {
    let encoded = std::str::from_utf8(blob).map_err(|_| "Backup file is corrupted")?;
    let encrypted = B64
        .decode(encoded)
        .map_err(|_| "Backup file is corrupted")?;
    let plaintext = crypto::open(key, &encrypted, aad)?;
    serde_json::from_slice(&plaintext).map_err(|_| "Backup file is corrupted".to_string())
}

fn file_map(files: &[FileMetadata]) -> HashMap<String, FileMetadata> {
    files
        .iter()
        .cloned()
        .map(|file| (file.name.clone(), file))
        .collect()
}

fn normalize_local_tombstones(payload: &mut Payload) {
    payload.entries.retain(|entry| {
        !payload
            .tombstones
            .get(&entry.id)
            .is_some_and(|deleted_at| *deleted_at >= entry.updated_at)
    });
    let live_updated: HashMap<_, _> = payload
        .entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry.updated_at))
        .collect();
    payload.tombstones.retain(|id, deleted_at| {
        live_updated
            .get(id.as_str())
            .is_none_or(|updated| *deleted_at >= *updated)
    });
}

async fn delete_named(
    client: &DriveClient,
    files: &HashMap<String, FileMetadata>,
    name: &str,
) -> Result<(), String> {
    if let Some(file) = files.get(name) {
        client.delete(&file.id).await?;
    }
    Ok(())
}

pub async fn sync_payload(
    client: &DriveClient,
    key: &[u8; 32],
    header: &mut Header,
    payload: &mut Payload,
) -> Result<SyncResult, String> {
    normalize_local_tombstones(payload);
    let mut files = client.list_files().await?;
    let mut duplicates = HashMap::<String, Vec<FileMetadata>>::new();
    for file in files.drain(..) {
        duplicates.entry(file.name.clone()).or_default().push(file);
    }
    let mut unique = Vec::new();
    let mut duplicate_files = Vec::new();
    for (_, mut versions) in duplicates {
        versions.sort_by_key(|file| property_time(Some(file)));
        if let Some(keep) = versions.pop() {
            duplicate_files.extend(versions);
            unique.push(keep);
        }
    }
    let mut files = file_map(&unique);

    if let Some(marker) = files.get("vault.chk") {
        let marker_data = client.download(&marker.id).await?;
        let marker_value: String = open_json(key, &marker_data, b"daylight-check-v1")
            .map_err(|_| "This Google Drive already holds a different Daylight vault. Restore from it on a fresh install, or disconnect Drive here.".to_string())?;
        if marker_value != "daylight" {
            return Err("This Google Drive already holds a different Daylight vault. Restore from it on a fresh install, or disconnect Drive here.".into());
        }
    } else if let Some(candidate) = unique
        .iter()
        .find(|file| file.name.starts_with("e-") || file.name.starts_with("t-"))
    {
        let blob = client.download(&candidate.id).await?;
        let valid = if let Some(id) = candidate.name.strip_prefix("e-") {
            open_json::<Entry>(key, &blob, format!("daylight-entry-v1:{id}").as_bytes())
                .is_ok_and(|entry| entry.id == id)
        } else if let Some(id) = candidate.name.strip_prefix("t-") {
            open_json::<Tombstone>(key, &blob, format!("daylight-tomb-v1:{id}").as_bytes())
                .is_ok_and(|tombstone| tombstone.id == id)
        } else {
            false
        };
        if !valid {
            return Err("This Google Drive already holds a different Daylight vault. Restore from it on a fresh install, or disconnect Drive here.".into());
        }
    }
    let tombstone_files: Vec<_> = files
        .iter()
        .filter_map(|(name, file)| {
            name.strip_prefix("t-")
                .map(|id| (name.clone(), id.to_string(), file.clone()))
        })
        .collect();
    for (name, id, file) in tombstone_files {
        let bytes = client.download(&file.id).await?;
        match open_json::<Tombstone>(key, &bytes, format!("daylight-tomb-v1:{id}").as_bytes()) {
            Ok(tombstone) if tombstone.id == id => {
                files.insert(
                    name,
                    FileMetadata {
                        id: file.id,
                        name: file.name,
                        app_properties: properties(
                            tombstone_stamp(&id, tombstone.deleted_at),
                            tombstone.deleted_at,
                        ),
                    },
                );
            }
            Ok(_) => return Err("Google Drive contains a mismatched tombstone".into()),
            Err(error) => return Err(format!("Google Drive tombstone is invalid: {error}")),
        }
    }

    if !files.contains_key("vault.chk") {
        let marker = seal_json(key, &"daylight", b"daylight-check-v1")?;
        let marker_stamp = stamp(&[b"chk"]);
        let id = client
            .upload(
                None,
                "vault.chk",
                "text/plain",
                &marker,
                &properties(marker_stamp, 0),
            )
            .await?;
        files.insert(
            "vault.chk".into(),
            FileMetadata {
                id,
                name: "vault.chk".into(),
                app_properties: properties(stamp(&[b"chk"]), 0),
            },
        );
    }

    if let Some(remote_header) = files.get("header.json") {
        let local_header = serde_json::to_vec(header).map_err(|e| e.to_string())?;
        let local_stamp = stamp(&[&local_header]);
        let remote_time = property_time(Some(remote_header));
        if remote_header.app_properties.get("h") != Some(&local_stamp)
            && remote_time > payload.header_at
        {
            let bytes = client.download(&remote_header.id).await?;
            let downloaded: Header = serde_json::from_slice(&bytes)
                .map_err(|_| "Drive key header is corrupted".to_string())?;
            *header = downloaded;
            payload.header_at = remote_time;
        } else if remote_header.app_properties.get("h") != Some(&local_stamp) {
            let props = properties(local_stamp, payload.header_at);
            client
                .upload(
                    Some(&remote_header.id),
                    "header.json",
                    "application/json",
                    &local_header,
                    &props,
                )
                .await?;
        }
    } else {
        let bytes = serde_json::to_vec(header).map_err(|e| e.to_string())?;
        let props = properties(stamp(&[&bytes]), payload.header_at);
        let id = client
            .upload(None, "header.json", "application/json", &bytes, &props)
            .await?;
        files.insert(
            "header.json".into(),
            FileMetadata {
                id,
                name: "header.json".into(),
                app_properties: props,
            },
        );
    }

    let mut ids = HashSet::new();
    ids.extend(payload.entries.iter().map(|entry| entry.id.clone()));
    ids.extend(payload.tombstones.keys().cloned());
    for name in files.keys() {
        if let Some(id) = name.strip_prefix("e-").or_else(|| name.strip_prefix("t-")) {
            ids.insert(id.to_string());
        }
    }

    let mut result = SyncResult {
        uploaded: 0,
        downloaded: 0,
        removed: 0,
        unchanged: 0,
        at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64,
    };
    let mut downloaded_entries = Vec::new();
    let mut adopted_tombstones = HashMap::new();
    let mut remove_local = HashSet::new();

    for id in ids {
        let entry_name = format!("e-{id}");
        let tomb_name = format!("t-{id}");
        let remote_entry = files.get(&entry_name);
        let remote_tomb = files.get(&tomb_name);
        let local_entry = payload.entries.iter().find(|entry| entry.id == id).cloned();
        let local_tomb = payload.tombstones.get(&id).copied();
        let local_stamp = local_entry.as_ref().map(entry_stamp);
        let local_live = local_entry.as_ref().map(|entry| ItemState::Live {
            updated: entry.updated_at,
            stamp: local_stamp.as_deref().unwrap_or_default(),
        });
        let remote_live_time = property_time(remote_entry);
        let remote_dead_time = property_time(remote_tomb);
        if remote_entry.is_some() && remote_tomb.is_some() {
            if remote_dead_time >= remote_live_time {
                delete_named(client, &files, &entry_name).await?;
            } else {
                delete_named(client, &files, &tomb_name).await?;
            }
        }

        let local_state = match (local_live, local_tomb) {
            (Some(ItemState::Live { updated, .. }), Some(deleted_at)) if deleted_at >= updated => {
                ItemState::Dead { deleted_at }
            }
            (Some(live), _) => live,
            (None, Some(deleted_at)) => ItemState::Dead { deleted_at },
            (None, None) => ItemState::Unknown,
        };
        let remote_state = match (remote_entry, remote_tomb) {
            (Some(_), Some(_)) if remote_dead_time >= remote_live_time => ItemState::Dead {
                deleted_at: remote_dead_time,
            },
            (Some(file), _) => ItemState::Live {
                updated: remote_live_time,
                stamp: file
                    .app_properties
                    .get("h")
                    .map(String::as_str)
                    .unwrap_or_default(),
            },
            (None, Some(_)) => ItemState::Dead {
                deleted_at: remote_dead_time,
            },
            (None, None) => ItemState::Unknown,
        };
        let actions = plan(local_state, remote_state);
        if actions.contains(&Action::Unchanged) {
            result.unchanged += 1;
        }
        if actions.contains(&Action::RemoveLocalEntry) {
            remove_local.insert(id.clone());
        }
        if actions.contains(&Action::AdoptTombstone) {
            let deleted_at = match remote_state {
                ItemState::Dead { deleted_at } => deleted_at,
                _ => unreachable!(),
            };
            if local_tomb != Some(deleted_at) {
                result.downloaded += 1;
            }
            adopted_tombstones.insert(id.clone(), deleted_at);
        }
        if actions.contains(&Action::UploadEntry) {
            if let Some(entry) = &local_entry {
                let blob = seal_json(key, entry, format!("daylight-entry-v1:{id}").as_bytes())?;
                let props = properties(entry_stamp(entry), entry.updated_at);
                client
                    .upload(
                        remote_entry.map(|file| file.id.as_str()),
                        &entry_name,
                        "text/plain",
                        &blob,
                        &props,
                    )
                    .await?;
                result.uploaded += 1;
                if remote_tomb.is_some() {
                    delete_named(client, &files, &tomb_name).await?;
                }
            }
        }
        if actions.contains(&Action::UploadTombstone) {
            if let Some(deleted_at) = local_tomb {
                let tombstone = Tombstone {
                    id: id.clone(),
                    deleted_at,
                };
                let blob = seal_json(key, &tombstone, format!("daylight-tomb-v1:{id}").as_bytes())?;
                let props = properties(tombstone_stamp(&id, deleted_at), deleted_at);
                client
                    .upload(
                        remote_tomb.map(|file| file.id.as_str()),
                        &tomb_name,
                        "text/plain",
                        &blob,
                        &props,
                    )
                    .await?;
                result.uploaded += 1;
                if remote_entry.is_some() {
                    delete_named(client, &files, &entry_name).await?;
                }
            }
        }
        if actions.contains(&Action::DeleteRemoteTombstone) {
            delete_named(client, &files, &tomb_name).await?;
        }
        if actions.contains(&Action::DeleteRemoteEntry) {
            delete_named(client, &files, &entry_name).await?;
        }
        if actions.contains(&Action::DownloadEntry) {
            if let Some(file) = remote_entry {
                let blob = client.download(&file.id).await?;
                match open_json::<Entry>(key, &blob, format!("daylight-entry-v1:{id}").as_bytes()) {
                    Ok(entry)
                        if entry.id == id
                            && property_time(Some(file)) == entry.updated_at
                            && file.app_properties.get("h") == Some(&entry_stamp(&entry)) =>
                    {
                        downloaded_entries.push(entry);
                        result.downloaded += 1;
                        if remote_tomb.is_some() {
                            delete_named(client, &files, &tomb_name).await?;
                        }
                    }
                    Ok(_) => {
                        return Err(
                            "Google Drive entry metadata does not match its encrypted data".into(),
                        )
                    }
                    Err(error) => return Err(format!("Google Drive entry is invalid: {error}")),
                }
            }
        }
        if actions.contains(&Action::DeleteRemoteTombstone)
            && !actions.contains(&Action::UploadEntry)
        {
            delete_named(client, &files, &tomb_name).await?;
        }
    }

    for id in remove_local {
        let existed = payload.entries.iter().any(|entry| entry.id == id);
        payload.entries.retain(|entry| entry.id != id);
        if existed {
            result.removed += 1;
        }
    }
    for (id, deleted_at) in adopted_tombstones {
        payload
            .tombstones
            .entry(id)
            .and_modify(|local| *local = (*local).max(deleted_at))
            .or_insert(deleted_at);
    }
    for entry in downloaded_entries {
        let protected = payload
            .tombstones
            .get(&entry.id)
            .is_some_and(|deleted_at| *deleted_at >= entry.updated_at);
        let locally_newer = payload
            .entries
            .iter()
            .find(|local| local.id == entry.id)
            .is_some_and(|local| local.updated_at > entry.updated_at);
        if protected || locally_newer {
            continue;
        }
        payload.tombstones.remove(&entry.id);
        if let Some(local) = payload
            .entries
            .iter_mut()
            .find(|local| local.id == entry.id)
        {
            *local = entry;
        } else {
            payload.entries.push(entry);
        }
    }
    payload.last_backup = result.at;
    for duplicate in duplicate_files {
        client.delete(&duplicate.id).await?;
    }
    Ok(result)
}

pub fn plan(local: ItemState<'_>, remote: ItemState<'_>) -> Vec<Action> {
    use Action::*;
    use ItemState::*;

    match (local, remote) {
        (Unknown, Unknown) | (Dead { .. }, Unknown) => Vec::new(),
        (Live { .. }, Unknown) => vec![UploadEntry],
        (Unknown, Live { .. }) => vec![DownloadEntry],
        (Unknown, Dead { .. }) => vec![AdoptTombstone],
        (Live { updated, .. }, Dead { deleted_at }) if deleted_at >= updated => {
            vec![RemoveLocalEntry, AdoptTombstone, DeleteRemoteEntry]
        }
        (Live { .. }, Dead { .. }) => vec![UploadEntry, DeleteRemoteTombstone],
        (Dead { deleted_at }, Live { updated, .. }) if deleted_at >= updated => {
            vec![UploadTombstone, DeleteRemoteEntry]
        }
        (Dead { .. }, Live { .. }) => vec![DownloadEntry],
        (Dead { deleted_at: local }, Dead { deleted_at: remote }) if local > remote => {
            vec![UploadTombstone]
        }
        (Dead { deleted_at: local }, Dead { deleted_at: remote }) if remote > local => {
            vec![AdoptTombstone]
        }
        (Dead { .. }, Dead { .. }) => vec![Unchanged],
        (
            Live {
                stamp: local_stamp, ..
            },
            Live {
                stamp: remote_stamp,
                ..
            },
        ) if local_stamp == remote_stamp => vec![Unchanged],
        (
            Live {
                updated: local_updated,
                stamp: local_stamp,
            },
            Live {
                updated: remote_updated,
                stamp: remote_stamp,
            },
        ) => {
            let ordering = local_updated
                .cmp(&remote_updated)
                .then_with(|| local_stamp.cmp(remote_stamp));
            match ordering {
                Ordering::Greater => vec![UploadEntry],
                Ordering::Less => vec![DownloadEntry],
                Ordering::Equal => vec![Unchanged],
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::Entry;

    #[test]
    fn local_tombstones_suppress_old_entries_and_expire_after_later_edits() {
        let mut payload = Payload {
            entries: vec![
                Entry {
                    id: "deleted".into(),
                    updated_at: 10,
                    ..Entry::default()
                },
                Entry {
                    id: "edited".into(),
                    updated_at: 12,
                    ..Entry::default()
                },
            ],
            tombstones: HashMap::from([
                ("deleted".into(), 10),
                ("edited".into(), 11),
                ("missing".into(), 9),
            ]),
            ..Payload::default()
        };

        normalize_local_tombstones(&mut payload);

        assert_eq!(
            payload
                .entries
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["edited"]
        );
        assert_eq!(payload.tombstones.get("deleted"), Some(&10));
        assert_eq!(payload.tombstones.get("edited"), None);
        assert_eq!(payload.tombstones.get("missing"), Some(&9));
    }

    #[test]
    fn local_edits_win_equal_timestamp_by_stamp() {
        assert_eq!(
            plan(
                ItemState::Live {
                    updated: 10,
                    stamp: "b"
                },
                ItemState::Live {
                    updated: 10,
                    stamp: "a"
                }
            ),
            vec![Action::UploadEntry]
        );
    }

    #[test]
    fn deletion_wins_equal_timestamp() {
        assert_eq!(
            plan(
                ItemState::Live {
                    updated: 10,
                    stamp: "a"
                },
                ItemState::Dead { deleted_at: 10 }
            ),
            vec![
                Action::RemoveLocalEntry,
                Action::AdoptTombstone,
                Action::DeleteRemoteEntry
            ]
        );
    }

    #[test]
    fn later_edit_resurrects_remote_tombstone() {
        assert_eq!(
            plan(
                ItemState::Live {
                    updated: 11,
                    stamp: "a"
                },
                ItemState::Dead { deleted_at: 10 }
            ),
            vec![Action::UploadEntry, Action::DeleteRemoteTombstone]
        );
    }

    #[test]
    fn newer_delete_is_adopted_or_uploaded_deterministically() {
        assert_eq!(
            plan(
                ItemState::Dead { deleted_at: 10 },
                ItemState::Dead { deleted_at: 11 }
            ),
            vec![Action::AdoptTombstone]
        );
        assert_eq!(
            plan(
                ItemState::Dead { deleted_at: 11 },
                ItemState::Dead { deleted_at: 10 }
            ),
            vec![Action::UploadTombstone]
        );
    }
}
