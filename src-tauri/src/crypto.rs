use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload as AeadPayload},
    XChaCha20Poly1305, XNonce,
};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zeroize::Zeroizing;

const WRAP_AAD: &[u8] = b"daylight-wrap-v1";
const VAULT_AAD: &[u8] = b"daylight-vault-v1";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;
const KEY_LEN: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    pub v: u8,
    pub salt: String,
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
    pub wrapped_key: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub body: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct DriveConfig {
    pub client_id: String,
    pub client_secret: String,
    pub refresh_token: String,
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Payload {
    pub entries: Vec<Entry>,
    pub drive: Option<DriveConfig>,
    pub last_backup: i64,
    pub tombstones: HashMap<String, i64>,
    pub header_at: i64,
}

#[derive(Serialize, Deserialize)]
pub struct VaultFile {
    pub header: Header,
    pub data: String,
}

fn derive_key(
    password: &str,
    salt: &[u8],
    memory: u32,
    passes: u32,
    lanes: u32,
) -> Result<Zeroizing<[u8; KEY_LEN]>, String> {
    let params = Params::new(memory, passes, lanes, Some(KEY_LEN)).map_err(|e| e.to_string())?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    argon
        .hash_password_into(password.as_bytes(), salt, &mut *key)
        .map_err(|e| e.to_string())?;
    Ok(key)
}

pub fn seal(key: &[u8; KEY_LEN], plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = XChaCha20Poly1305::new_from_slice(key).map_err(|e| e.to_string())?;
    let mut nonce = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    let ciphertext = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            AeadPayload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| "Encryption failed".to_string())?;
    let mut blob = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&ciphertext);
    Ok(blob)
}

pub fn open(key: &[u8; KEY_LEN], blob: &[u8], aad: &[u8]) -> Result<Vec<u8>, String> {
    if blob.len() < NONCE_LEN + 16 {
        return Err("Encrypted data is too short".into());
    }
    let cipher = XChaCha20Poly1305::new_from_slice(key).map_err(|e| e.to_string())?;
    cipher
        .decrypt(
            XNonce::from_slice(&blob[..NONCE_LEN]),
            AeadPayload {
                msg: &blob[NONCE_LEN..],
                aad,
            },
        )
        .map_err(|_| "Wrong password or corrupted vault".to_string())
}

pub fn create(
    password: &str,
    payload: &Payload,
) -> Result<(Header, Zeroizing<[u8; KEY_LEN]>, String), String> {
    let mut dek = Zeroizing::new([0u8; KEY_LEN]);
    OsRng.fill_bytes(&mut *dek);
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let kek = derive_key(password, &salt, 65_536, 3, 1)?;
    let wrapped_key = seal(&kek, &*dek, WRAP_AAD)?;
    let header = Header {
        v: 1,
        salt: B64.encode(salt),
        m_kib: 65_536,
        t: 3,
        p: 1,
        wrapped_key: B64.encode(wrapped_key),
    };
    let plaintext = serde_json::to_vec(payload).map_err(|e| e.to_string())?;
    let data = B64.encode(seal(&dek, &plaintext, VAULT_AAD)?);
    Ok((header, dek, data))
}

pub fn unlock(
    header: &Header,
    data: &str,
    password: &str,
) -> Result<(Zeroizing<[u8; KEY_LEN]>, Payload), String> {
    let dek = unwrap_key(header, password)?;
    let payload = unlock_with_key(&dek, data)?;
    Ok((dek, payload))
}

pub fn unlock_with_key(dek: &[u8; KEY_LEN], data: &str) -> Result<Payload, String> {
    let encrypted = B64.decode(data).map_err(|_| "Vault file is corrupted")?;
    let plaintext = open(&dek, &encrypted, VAULT_AAD)?;
    let payload = serde_json::from_slice(&plaintext).map_err(|_| "Vault file is corrupted")?;
    Ok(payload)
}

pub fn unwrap_key(header: &Header, password: &str) -> Result<Zeroizing<[u8; KEY_LEN]>, String> {
    if header.v != 1
        || header.m_kib > 1_048_576
        || header.t > 16
        || header.p > 16
        || header.m_kib < 8
        || header.t == 0
        || header.p == 0
    {
        return Err("Unsupported or unsafe vault header".into());
    }
    let salt = B64
        .decode(&header.salt)
        .map_err(|_| "Vault file is corrupted")?;
    if salt.len() != SALT_LEN {
        return Err("Vault file is corrupted".into());
    }
    let kek = derive_key(password, &salt, header.m_kib, header.t, header.p)?;
    let wrapped = B64
        .decode(&header.wrapped_key)
        .map_err(|_| "Vault file is corrupted")?;
    let dek_vec = open(&kek, &wrapped, WRAP_AAD).map_err(|_| "Wrong password".to_string())?;
    let dek_array: [u8; KEY_LEN] = dek_vec.try_into().map_err(|_| "Vault file is corrupted")?;
    Ok(Zeroizing::new(dek_array))
}

pub fn rewrap(dek: &[u8; KEY_LEN], password: &str) -> Result<Header, String> {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let kek = derive_key(password, &salt, 65_536, 3, 1)?;
    Ok(Header {
        v: 1,
        salt: B64.encode(salt),
        m_kib: 65_536,
        t: 3,
        p: 1,
        wrapped_key: B64.encode(seal(&kek, dek, WRAP_AAD)?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_wrap_round_trips_payload() {
        let payload = Payload {
            entries: vec![Entry {
                id: "entry-1".into(),
                title: "A private thought".into(),
                body: "hello".into(),
                created_at: 1,
                updated_at: 1,
            }],
            ..Payload::default()
        };
        let (header, _, data) = create("correct horse battery staple", &payload).unwrap();
        let (_, restored) = unlock(&header, &data, "correct horse battery staple").unwrap();
        assert_eq!(restored.entries[0].body, "hello");
        assert!(unlock(&header, &data, "not the password").is_err());
    }

    #[test]
    fn older_payloads_default_new_fields() {
        let payload: Payload = serde_json::from_str(r#"{"entries":[]}"#).unwrap();
        assert!(payload.drive.is_none());
        assert_eq!(payload.last_backup, 0);
        assert!(payload.tombstones.is_empty());
        assert_eq!(payload.header_at, 0);
    }
}
