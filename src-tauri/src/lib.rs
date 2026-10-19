mod biometric;
mod crypto;
mod drive;
mod keystore;
mod sync;

use crypto::{DriveConfig, Entry, Header, Payload, VaultFile};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager, State};
use zeroize::Zeroizing;

struct Session {
    dek: Zeroizing<[u8; 32]>,
    header: Header,
    payload: Payload,
}

struct AppState {
    dir: PathBuf,
    session: Option<Session>,
    syncing: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    exists: bool,
    unlocked: bool,
    biometric_available: bool,
    biometric_enabled: bool,
    biometric_error: Option<String>,
    biometric_method: &'static str,
    drive_connected: bool,
    last_backup: Option<i64>,
}

fn lock_state(state: &Mutex<AppState>) -> Result<MutexGuard<'_, AppState>, String> {
    state
        .lock()
        .map_err(|_| "Application state is unavailable".to_string())
}

struct SyncReset<'a>(&'a Mutex<AppState>);

#[derive(Deserialize)]
struct DriveTombstone {
    id: String,
    deleted_at: i64,
}

impl Drop for SyncReset<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.lock() {
            state.syncing = false;
        }
    }
}

fn vault_path(dir: &Path) -> PathBuf {
    dir.join("vault.dat")
}

fn biometric_method() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(any(target_os = "android", target_os = "ios")) {
        "mobile"
    } else {
        "none"
    }
}

fn filter_restored_entries(entries: Vec<Entry>, tombstones: &HashMap<String, i64>) -> Vec<Entry> {
    entries
        .into_iter()
        .filter(|entry| {
            tombstones
                .get(&entry.id)
                .is_none_or(|deleted_at| *deleted_at < entry.updated_at)
        })
        .collect()
}

fn legacy_import(dir: &Path) -> Result<(Payload, Option<(PathBuf, PathBuf)>), String> {
    let source = dir.join("diary.json");
    let backup = dir.join("diary.json.plain-backup");
    let (input, rename_source) = if source.exists() {
        if backup.exists() {
            return Err(
                "A legacy backup already exists; move it before importing diary.json".into(),
            );
        }
        (source.clone(), true)
    } else if backup.exists() {
        (backup, false)
    } else {
        return Ok((Payload::default(), None));
    };
    let bytes = fs::read(&input).map_err(|e| format!("Couldn't read legacy journal: {e}"))?;
    let mut entries: Vec<Entry> =
        serde_json::from_slice(&bytes).map_err(|e| format!("Legacy journal is invalid: {e}"))?;
    let mut ids = HashSet::new();
    for entry in &mut entries {
        if entry.id.trim().is_empty() {
            entry.id = uuid::Uuid::new_v4().to_string();
        }
        if !ids.insert(entry.id.clone()) {
            return Err(format!(
                "Legacy journal contains duplicate entry id: {}",
                entry.id
            ));
        }
        if entry.updated_at == 0 {
            entry.updated_at = entry.created_at;
        }
    }
    let rename = rename_source.then_some((source, dir.join("diary.json.plain-backup")));
    Ok((
        Payload {
            entries,
            ..Payload::default()
        },
        rename,
    ))
}

fn read_vault(path: &Path) -> Result<VaultFile, String> {
    let bytes = fs::read(path).map_err(|e| format!("Couldn't read vault: {e}"))?;
    serde_json::from_slice(&bytes).map_err(|_| "Vault file is corrupted".to_string())
}

fn persist(path: &Path, session: &Session) -> Result<(), String> {
    let plaintext = serde_json::to_vec(&session.payload).map_err(|e| e.to_string())?;
    let encrypted = crypto::seal(&session.dek, &plaintext, b"daylight-vault-v1")?;
    let vault = VaultFile {
        header: session.header.clone(),
        data: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, encrypted),
    };
    let bytes = serde_json::to_vec(&vault).map_err(|e| e.to_string())?;
    let parent = path
        .parent()
        .ok_or_else(|| "Vault directory is invalid".to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| format!("Couldn't create temporary vault: {e}"))?;
    temp.write_all(&bytes)
        .map_err(|e| format!("Couldn't write vault: {e}"))?;
    temp.as_file()
        .sync_all()
        .map_err(|e| format!("Couldn't flush vault: {e}"))?;
    temp.persist(path)
        .map_err(|e| format!("Couldn't replace vault: {}", e.error))?;
    Ok(())
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn password_valid(password: &str) -> Result<(), String> {
    if password.chars().count() < 8 {
        return Err("Password must be at least 8 characters".into());
    }
    Ok(())
}

#[tauri::command]
async fn vault_status(
    #[cfg_attr(not(mobile), allow(unused_variables))] app: AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<Status, String> {
    let (dir, unlocked, drive_connected, last_backup) = {
        let state = lock_state(&state)?;
        let session = state.session.as_ref();
        (
            state.dir.clone(),
            session.is_some(),
            session.is_some_and(|session| session.payload.drive.is_some()),
            session.and_then(|session| {
                (session.payload.last_backup > 0).then_some(session.payload.last_backup)
            }),
        )
    };
    #[cfg(windows)]
    let (biometric_available, availability_error) = match biometric::available().await {
        Ok(available) => (available, None),
        Err(error) => {
            eprintln!("Windows Hello availability check failed: {error}");
            (false, Some(error))
        }
    };
    #[cfg(mobile)]
    let (biometric_available, availability_error) = match biometric::mobile::available(&app) {
        Ok(available) => (available, None),
        Err(error) => {
            eprintln!("Biometric availability check failed: {error}");
            (false, Some(error))
        }
    };
    #[cfg(not(any(windows, mobile)))]
    let (biometric_available, availability_error) = (false, None);
    let (biometric_enabled, key_error) = match keystore::exists(&dir) {
        Ok(enabled) => (enabled, None),
        Err(error) => {
            eprintln!("Biometric key availability check failed: {error}");
            (false, Some(error))
        }
    };
    let biometric_error = availability_error.or(key_error);
    Ok(Status {
        exists: vault_path(&dir).exists(),
        unlocked,
        biometric_available,
        biometric_enabled,
        biometric_error,
        biometric_method: biometric_method(),
        drive_connected,
        last_backup,
    })
}

#[tauri::command]
async fn setup_vault(state: State<'_, Mutex<AppState>>, password: String) -> Result<(), String> {
    password_valid(&password)?;
    let (dir, path) = {
        let state = lock_state(&state)?;
        let path = vault_path(&state.dir);
        if path.exists() {
            return Err("A vault already exists".into());
        }
        (state.dir.clone(), path)
    };
    let (payload, legacy_backup) = legacy_import(&dir)?;
    let payload_for_work = payload.clone();
    let (header, dek, data) =
        tauri::async_runtime::spawn_blocking(move || crypto::create(&password, &payload_for_work))
            .await
            .map_err(|e| format!("Password setup failed: {e}"))??;
    let mut state = lock_state(&state)?;
    if path.exists() {
        return Err("A vault already exists".into());
    }
    let session = Session {
        dek,
        header,
        payload,
    };
    let vault = VaultFile {
        header: session.header.clone(),
        data,
    };
    let bytes = serde_json::to_vec(&vault).map_err(|e| e.to_string())?;
    let parent = path
        .parent()
        .ok_or_else(|| "Vault directory is invalid".to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| format!("Couldn't create temporary vault: {e}"))?;
    temp.write_all(&bytes)
        .map_err(|e| format!("Couldn't write vault: {e}"))?;
    temp.as_file()
        .sync_all()
        .map_err(|e| format!("Couldn't flush vault: {e}"))?;
    if let Some((source, backup)) = &legacy_backup {
        fs::rename(source, backup)
            .map_err(|e| format!("Couldn't preserve the legacy journal: {e}"))?;
    }
    if let Err(error) = temp.persist_noclobber(&path) {
        let persist_error = format!("Couldn't create vault: {}", error.error);
        if let Some((source, backup)) = &legacy_backup {
            if let Err(restore_error) = fs::rename(backup, source) {
                return Err(format!(
                    "{persist_error}; couldn't restore legacy journal: {restore_error}"
                ));
            }
        }
        return Err(persist_error);
    }
    state.session = Some(session);
    Ok(())
}

#[tauri::command]
async fn unlock(state: State<'_, Mutex<AppState>>, password: String) -> Result<(), String> {
    let path = {
        let state = lock_state(&state)?;
        vault_path(&state.dir)
    };
    let vault = read_vault(&path)?;
    let header = vault.header;
    let session_header = header.clone();
    let data = vault.data;
    let (dek, payload) =
        tauri::async_runtime::spawn_blocking(move || crypto::unlock(&header, &data, &password))
            .await
            .map_err(|e| format!("Unlock failed: {e}"))??;
    lock_state(&state)?.session = Some(Session {
        dek,
        header: session_header,
        payload,
    });
    Ok(())
}

#[tauri::command]
async fn biometric_unlock(
    #[cfg_attr(not(any(windows, mobile)), allow(unused_variables))] app: AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let (dir, path) = {
        let state = lock_state(&state)?;
        if state.session.is_some() {
            return Err("The journal is already unlocked".into());
        }
        (state.dir.clone(), vault_path(&state.dir))
    };
    #[cfg(windows)]
    let hwnd = app
        .get_webview_window("main")
        .ok_or_else(|| "Daylight window is unavailable".to_string())?
        .hwnd()
        .map_err(|error| format!("Couldn't access the Daylight window: {error}"))?
        .0 as isize;
    #[cfg(windows)]
    biometric::verify(hwnd).await?;
    #[cfg(mobile)]
    biometric::mobile::verify(&app, "Unlock Daylight").await?;
    let dek = keystore::load(&dir)?;
    let vault = read_vault(&path)?;
    let payload = match crypto::unlock_with_key(&dek, &vault.data) {
        Ok(payload) => payload,
        Err(_) => {
            keystore::remove(&dir)?;
            return Err("Biometric unlock is out of date. Use your password.".into());
        }
    };
    let mut state = lock_state(&state)?;
    if state.session.is_some() {
        return Err("The journal was unlocked while biometric verification was running".into());
    }
    state.session = Some(Session {
        dek,
        header: vault.header,
        payload,
    });
    Ok(())
}

#[tauri::command]
async fn enable_biometric_unlock(
    #[cfg_attr(not(any(windows, mobile)), allow(unused_variables))] app: AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let (dir, dek) = {
        let state = lock_state(&state)?;
        let session = state.session.as_ref().ok_or_else(|| "Locked".to_string())?;
        (state.dir.clone(), Zeroizing::new(*session.dek))
    };
    #[cfg(windows)]
    let hwnd = app
        .get_webview_window("main")
        .ok_or_else(|| "Daylight window is unavailable".to_string())?
        .hwnd()
        .map_err(|error| format!("Couldn't access the Daylight window: {error}"))?
        .0 as isize;
    #[cfg(windows)]
    biometric::verify(hwnd).await?;
    #[cfg(mobile)]
    biometric::mobile::verify(&app, "Enable biometric unlock for Daylight").await?;
    let state = lock_state(&state)?;
    let session = state.session.as_ref().ok_or_else(|| "Locked".to_string())?;
    if session.dek[..] != dek[..] {
        return Err("Vault session changed; try again".into());
    }
    keystore::store(&dir, &dek)
}

#[tauri::command]
fn disable_biometric_unlock(state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let state = lock_state(&state)?;
    if state.session.is_none() {
        return Err("Locked".into());
    }
    keystore::remove(&state.dir)
}

#[tauri::command]
fn lock(state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    lock_state(&state)?.session = None;
    Ok(())
}

#[tauri::command]
fn list_entries(state: State<'_, Mutex<AppState>>) -> Result<Vec<Entry>, String> {
    let state = lock_state(&state)?;
    let session = state.session.as_ref().ok_or_else(|| "Locked".to_string())?;
    let mut entries = session.payload.entries.clone();
    entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(entries)
}

#[tauri::command(rename_all = "camelCase")]
fn create_entry(
    state: State<'_, Mutex<AppState>>,
    title: String,
    body: String,
    id: Option<String>,
    created_at: Option<i64>,
) -> Result<Entry, String> {
    let mut state = lock_state(&state)?;
    let path = vault_path(&state.dir);
    let session = state.session.as_mut().ok_or_else(|| "Locked".to_string())?;
    let id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    if session.payload.entries.iter().any(|entry| entry.id == id) {
        return Err("Entry already exists".into());
    }
    let now = now_seconds();
    let entry = Entry {
        id,
        title,
        body,
        created_at: created_at.unwrap_or(now),
        updated_at: now,
    };
    let previous_tombstone = session.payload.tombstones.remove(&entry.id);
    session.payload.entries.push(entry.clone());
    if let Err(error) = persist(&path, session) {
        session.payload.entries.retain(|item| item.id != entry.id);
        if let Some(deleted_at) = previous_tombstone {
            session
                .payload
                .tombstones
                .insert(entry.id.clone(), deleted_at);
        }
        return Err(error);
    }
    Ok(entry)
}

#[tauri::command(rename_all = "camelCase")]
fn update_entry(
    state: State<'_, Mutex<AppState>>,
    id: String,
    title: String,
    body: String,
) -> Result<Entry, String> {
    let mut state = lock_state(&state)?;
    let path = vault_path(&state.dir);
    let session = state.session.as_mut().ok_or_else(|| "Locked".to_string())?;
    let entry = session
        .payload
        .entries
        .iter_mut()
        .find(|entry| entry.id == id)
        .ok_or_else(|| "Entry not found".to_string())?;
    let previous = entry.clone();
    entry.title = title;
    entry.body = body;
    entry.updated_at = now_seconds();
    let updated = entry.clone();
    if let Err(error) = persist(&path, session) {
        if let Some(entry) = session
            .payload
            .entries
            .iter_mut()
            .find(|entry| entry.id == id)
        {
            *entry = previous;
        }
        return Err(error);
    }
    Ok(updated)
}

#[tauri::command]
fn delete_entry(state: State<'_, Mutex<AppState>>, id: String) -> Result<(), String> {
    let mut state = lock_state(&state)?;
    let path = vault_path(&state.dir);
    let session = state.session.as_mut().ok_or_else(|| "Locked".to_string())?;
    let index = session
        .payload
        .entries
        .iter()
        .position(|entry| entry.id == id)
        .ok_or_else(|| "Entry not found".to_string())?;
    let deleted = session.payload.entries.remove(index);
    let old_tombstone = session.payload.tombstones.remove(&id);
    if session.payload.drive.is_some() {
        session.payload.tombstones.insert(id.clone(), now_seconds());
    } else if let Some(deleted_at) = old_tombstone {
        session.payload.tombstones.insert(id.clone(), deleted_at);
    }
    if let Err(error) = persist(&path, session) {
        session.payload.entries.insert(index, deleted);
        session.payload.tombstones.remove(&id);
        if let Some(deleted_at) = old_tombstone {
            session.payload.tombstones.insert(id, deleted_at);
        }
        return Err(error);
    }
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
async fn change_password(
    state: State<'_, Mutex<AppState>>,
    old_password: String,
    new_password: String,
) -> Result<(), String> {
    password_valid(&new_password)?;
    let (path, old_header, dek, data, previous_header) = {
        let state = lock_state(&state)?;
        let path = vault_path(&state.dir);
        let vault = read_vault(&path)?;
        let session = state.session.as_ref().ok_or_else(|| "Locked".to_string())?;
        (
            path,
            vault.header,
            *session.dek,
            vault.data,
            session.header.clone(),
        )
    };
    let new_password_for_work = new_password.clone();
    let (verified_dek, header) = tauri::async_runtime::spawn_blocking(move || {
        let (verified_dek, _) = crypto::unlock(&old_header, &data, &old_password)?;
        if verified_dek[..] != dek[..] {
            return Err("Password does not match the current session".to_string());
        }
        let header = crypto::rewrap(&verified_dek, &new_password_for_work)?;
        Ok::<_, String>((verified_dek, header))
    })
    .await
    .map_err(|e| format!("Password change failed: {e}"))??;
    let mut state = lock_state(&state)?;
    let session = state.session.as_mut().ok_or_else(|| "Locked".to_string())?;
    if session.dek[..] != verified_dek[..] {
        return Err("Vault session changed; try again".into());
    }
    session.header = header;
    let previous_header_at = session.payload.header_at;
    session.payload.header_at = now_seconds();
    if let Err(error) = persist(&path, session) {
        session.header = previous_header;
        session.payload.header_at = previous_header_at;
        return Err(error);
    }
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
async fn drive_connect(
    state: State<'_, Mutex<AppState>>,
    client_id: String,
    client_secret: String,
) -> Result<(), String> {
    let client_id = client_id.trim().to_string();
    let client_secret = client_secret.trim().to_string();
    if client_id.is_empty() || client_secret.is_empty() {
        return Err("Enter both the client ID and client secret".into());
    }
    let dek = {
        let state = lock_state(&state)?;
        let session = state.session.as_ref().ok_or_else(|| "Locked".to_string())?;
        *session.dek
    };
    let refresh_token = drive::oauth_connect(&client_id, &client_secret).await?;
    let mut state = lock_state(&state)?;
    let path = vault_path(&state.dir);
    let session = state.session.as_mut().ok_or_else(|| "Locked".to_string())?;
    if session.dek[..] != dek[..] {
        return Err("Vault session changed; try again".into());
    }
    let previous = session.payload.drive.clone();
    session.payload.drive = Some(DriveConfig {
        client_id,
        client_secret,
        refresh_token,
    });
    if let Err(error) = persist(&path, session) {
        session.payload.drive = previous;
        return Err(error);
    }
    Ok(())
}

#[tauri::command]
fn drive_disconnect(state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let mut state = lock_state(&state)?;
    let path = vault_path(&state.dir);
    let session = state.session.as_mut().ok_or_else(|| "Locked".to_string())?;
    let previous = session.payload.drive.take();
    if let Err(error) = persist(&path, session) {
        session.payload.drive = previous;
        return Err(error);
    }
    Ok(())
}

#[tauri::command]
async fn sync_now(state: State<'_, Mutex<AppState>>) -> Result<sync::SyncResult, String> {
    let (path, dek, snapshot_header, mut header, snapshot, config) = {
        let mut state = lock_state(&state)?;
        if state.syncing {
            return Err("A Google Drive sync is already running".into());
        }
        let session = state.session.as_ref().ok_or_else(|| "Locked".to_string())?;
        let config = session
            .payload
            .drive
            .clone()
            .ok_or_else(|| "Google Drive is not connected".to_string())?;
        state.syncing = true;
        let session = state.session.as_ref().ok_or_else(|| "Locked".to_string())?;
        (
            vault_path(&state.dir),
            *session.dek,
            session.header.clone(),
            session.header.clone(),
            session.payload.clone(),
            config,
        )
    };
    let _sync_reset = SyncReset(&state);
    let client = drive::DriveClient::new(
        &config.client_id,
        &config.client_secret,
        &config.refresh_token,
    )
    .await?;
    let mut synced = snapshot.clone();
    let result = sync::sync_payload(&client, &dek, &mut header, &mut synced).await?;
    let mut state = lock_state(&state)?;
    let session = state.session.as_mut().ok_or_else(|| "Locked".to_string())?;
    if session.dek[..] != dek[..] {
        return Err("Vault session changed during sync".into());
    }
    if session.header != snapshot_header {
        header = session.header.clone();
        synced.header_at = session.payload.header_at;
    }

    let current = &session.payload;
    let snapshot_entries: std::collections::HashMap<_, _> = snapshot
        .entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect();
    let mut entries: std::collections::HashMap<String, Entry> = synced
        .entries
        .drain(..)
        .map(|entry| (entry.id.clone(), entry))
        .collect();
    for current_entry in &current.entries {
        let changed_during_sync = snapshot_entries
            .get(current_entry.id.as_str())
            .is_none_or(|before| *before != current_entry);
        if changed_during_sync {
            entries.insert(current_entry.id.clone(), current_entry.clone());
            synced.tombstones.remove(&current_entry.id);
        }
    }
    for id in current.tombstones.keys() {
        if !snapshot.tombstones.contains_key(id) || current.tombstones[id] > snapshot.tombstones[id]
        {
            entries.remove(id);
        }
    }
    synced.entries = entries.into_values().collect();
    synced
        .entries
        .sort_by(|a, b| b.created_at.cmp(&a.created_at));
    for (id, deleted_at) in &current.tombstones {
        synced
            .tombstones
            .entry(id.clone())
            .and_modify(|remote| *remote = (*remote).max(*deleted_at))
            .or_insert(*deleted_at);
    }
    synced.drive = current.drive.clone();
    let previous_header = session.header.clone();
    let previous_payload = session.payload.clone();
    session.header = header;
    session.payload = synced;
    if let Err(error) = persist(&path, session) {
        session.header = previous_header;
        session.payload = previous_payload;
        return Err(error);
    }
    Ok(result)
}

#[tauri::command(rename_all = "camelCase")]
async fn restore_from_drive(
    state: State<'_, Mutex<AppState>>,
    client_id: String,
    client_secret: String,
    password: String,
) -> Result<usize, String> {
    password_valid(&password)?;
    let client_id = client_id.trim().to_string();
    let client_secret = client_secret.trim().to_string();
    if client_id.is_empty() || client_secret.is_empty() {
        return Err("Enter both the client ID and client secret".into());
    }
    let (dir, path) = {
        let state = lock_state(&state)?;
        let path = vault_path(&state.dir);
        if path.exists() {
            return Err("Restore is only available when no vault exists".into());
        }
        (state.dir.clone(), path)
    };
    let refresh_token = drive::oauth_connect(&client_id, &client_secret).await?;
    let client = drive::DriveClient::new(&client_id, &client_secret, &refresh_token).await?;
    let files = client.list_files().await?;
    let remote_header = files
        .iter()
        .filter(|file| file.name == "header.json")
        .max_by_key(|file| {
            file.app_properties
                .get("u")
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or_default()
        })
        .ok_or_else(|| "No Daylight backup found in this Google account".to_string())?;
    let header_bytes = client.download(&remote_header.id).await?;
    let header: Header =
        serde_json::from_slice(&header_bytes).map_err(|_| "Drive key header is corrupted")?;
    let dek = tauri::async_runtime::spawn_blocking({
        let header = header.clone();
        let password = password.clone();
        move || crypto::unwrap_key(&header, &password)
    })
    .await
    .map_err(|e| format!("Restore failed: {e}"))??;

    if let Some(marker) = files.iter().find(|file| file.name == "vault.chk") {
        let marker_bytes = client.download(&marker.id).await?;
        let encoded =
            std::str::from_utf8(&marker_bytes).map_err(|_| "Backup marker is corrupted")?;
        let encrypted = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
            .map_err(|_| "Backup marker is corrupted")?;
        let plaintext = crypto::open(&dek, &encrypted, b"daylight-check-v1")?;
        if serde_json::from_slice::<String>(&plaintext).ok().as_deref() != Some("daylight") {
            return Err("This Google Drive already holds a different Daylight vault".into());
        }
    }

    let mut entries = Vec::new();
    let mut latest_entries = HashMap::<String, &drive::FileMetadata>::new();
    for file in files.iter().filter(|file| file.name.starts_with("e-")) {
        let Some(id) = file.name.strip_prefix("e-") else {
            continue;
        };
        if id.is_empty() {
            continue;
        }
        let updated = file
            .app_properties
            .get("u")
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or_default();
        let replace = latest_entries.get(id).is_none_or(|current| {
            let current_updated = current
                .app_properties
                .get("u")
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or_default();
            (current_updated, current.id.as_str()) < (updated, file.id.as_str())
        });
        if replace {
            latest_entries.insert(id.to_string(), file);
        }
    }
    let mut latest_tombstones = HashMap::<String, &drive::FileMetadata>::new();
    for file in files.iter().filter(|file| file.name.starts_with("t-")) {
        let Some(id) = file.name.strip_prefix("t-") else {
            continue;
        };
        if id.is_empty() {
            continue;
        }
        let updated = file
            .app_properties
            .get("u")
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or_default();
        let replace = latest_tombstones.get(id).is_none_or(|current| {
            let current_updated = current
                .app_properties
                .get("u")
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or_default();
            (current_updated, current.id.as_str()) < (updated, file.id.as_str())
        });
        if replace {
            latest_tombstones.insert(id.to_string(), file);
        }
    }
    let mut tombstones = HashMap::new();
    for (id, file) in latest_tombstones {
        let blob = client.download(&file.id).await?;
        let encoded = std::str::from_utf8(&blob).map_err(|_| "Backup tombstone is corrupted")?;
        let encrypted = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
            .map_err(|_| "Backup tombstone is corrupted")?;
        let plaintext = crypto::open(
            &dek,
            &encrypted,
            format!("daylight-tomb-v1:{id}").as_bytes(),
        )?;
        let tombstone: DriveTombstone =
            serde_json::from_slice(&plaintext).map_err(|_| "Backup tombstone is corrupted")?;
        let metadata_time = file
            .app_properties
            .get("u")
            .and_then(|value| value.parse::<i64>().ok());
        let metadata_stamp = file.app_properties.get("h");
        if tombstone.id != id
            || metadata_time != Some(tombstone.deleted_at)
            || metadata_stamp != Some(&sync::tombstone_stamp(&id, tombstone.deleted_at))
        {
            return Err("Backup contains a mismatched tombstone".into());
        }
        tombstones.insert(id, tombstone.deleted_at);
    }
    for (id, file) in latest_entries {
        let blob = client.download(&file.id).await?;
        let encoded = std::str::from_utf8(&blob).map_err(|_| "Backup entry is corrupted")?;
        let encrypted = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
            .map_err(|_| "Backup entry is corrupted")?;
        let plaintext = crypto::open(
            &dek,
            &encrypted,
            format!("daylight-entry-v1:{id}").as_bytes(),
        )?;
        let entry: Entry =
            serde_json::from_slice(&plaintext).map_err(|_| "Backup entry is corrupted")?;
        let metadata_time = file
            .app_properties
            .get("u")
            .and_then(|value| value.parse::<i64>().ok());
        let metadata_stamp = file.app_properties.get("h");
        if entry.id != id
            || metadata_time != Some(entry.updated_at)
            || metadata_stamp != Some(&sync::entry_stamp(&entry))
        {
            return Err("Backup contains a mismatched entry".into());
        }
        entries.push(entry);
    }
    let entries = filter_restored_entries(entries, &tombstones);
    let payload = Payload {
        entries,
        tombstones,
        drive: Some(DriveConfig {
            client_id,
            client_secret,
            refresh_token,
        }),
        header_at: remote_header
            .app_properties
            .get("u")
            .and_then(|value| value.parse().ok())
            .unwrap_or_default(),
        ..Payload::default()
    };
    let count = payload.entries.len();
    let plaintext = serde_json::to_vec(&payload).map_err(|e| e.to_string())?;
    let encrypted = crypto::seal(&dek, &plaintext, b"daylight-vault-v1")?;
    let vault = VaultFile {
        header: header.clone(),
        data: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, encrypted),
    };
    let bytes = serde_json::to_vec(&vault).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(&dir)
        .map_err(|e| format!("Couldn't create temporary vault: {e}"))?;
    temp.write_all(&bytes)
        .map_err(|e| format!("Couldn't write vault: {e}"))?;
    temp.as_file()
        .sync_all()
        .map_err(|e| format!("Couldn't flush vault: {e}"))?;

    let mut state = lock_state(&state)?;
    if path.exists() {
        return Err("A vault already exists".into());
    }
    temp.persist_noclobber(&path)
        .map_err(|e| format!("Couldn't create vault: {}", e.error))?;
    state.session = Some(Session {
        dek,
        header,
        payload,
    });
    Ok(count)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_biometric::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            fs::create_dir_all(&dir)?;
            app.manage(Mutex::new(AppState {
                dir,
                session: None,
                syncing: false,
            }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            vault_status,
            setup_vault,
            unlock,
            biometric_unlock,
            enable_biometric_unlock,
            disable_biometric_unlock,
            lock,
            list_entries,
            create_entry,
            update_entry,
            delete_entry,
            change_password,
            drive_connect,
            drive_disconnect,
            sync_now,
            restore_from_drive
        ])
        .run(tauri::generate_context!())
        .expect("error while running Daylight");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_does_not_resurrect_deleted_entries() {
        let entries = vec![
            Entry {
                id: "deleted-at-same-time".into(),
                updated_at: 10,
                ..Entry::default()
            },
            Entry {
                id: "edited-after-delete".into(),
                updated_at: 12,
                ..Entry::default()
            },
            Entry {
                id: "not-deleted".into(),
                updated_at: 8,
                ..Entry::default()
            },
        ];
        let tombstones = HashMap::from([
            ("deleted-at-same-time".into(), 10),
            ("edited-after-delete".into(), 11),
        ]);

        let restored = filter_restored_entries(entries, &tombstones);

        assert_eq!(
            restored
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["edited-after-delete", "not-deleted"]
        );
    }

    #[test]
    fn legacy_import_loads_entries_and_plans_plain_backup() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("diary.json");
        fs::write(
            &source,
            r#"[{"title":"Imported","body":"A note","created_at":42}]"#,
        )
        .unwrap();

        let (payload, backup) = legacy_import(dir.path()).unwrap();

        assert_eq!(payload.entries.len(), 1);
        assert_eq!(payload.entries[0].title, "Imported");
        assert_eq!(payload.entries[0].created_at, 42);
        assert_eq!(payload.entries[0].updated_at, 42);
        assert!(!payload.entries[0].id.is_empty());
        assert_eq!(
            backup,
            Some((source.clone(), dir.path().join("diary.json.plain-backup")))
        );
        assert!(source.exists());
    }

    #[test]
    fn legacy_import_rejects_invalid_json_without_changing_source() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("diary.json");
        fs::write(&source, "not json").unwrap();

        assert!(legacy_import(dir.path()).is_err());
        assert_eq!(fs::read_to_string(source).unwrap(), "not json");
    }
}
