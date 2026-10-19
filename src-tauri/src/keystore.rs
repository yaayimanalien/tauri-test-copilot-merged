use std::path::Path;
use zeroize::{Zeroize, Zeroizing};

#[cfg(any(windows, target_os = "ios"))]
const SERVICE: &str = "com.daylight.diary";
#[cfg(any(windows, target_os = "ios"))]
const USER: &str = "vault-key";

#[cfg(any(windows, target_os = "ios"))]
fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, USER).map_err(|e| format!("Couldn't open the OS keyring: {e}"))
}

#[cfg(any(windows, target_os = "ios"))]
pub fn store(_dir: &Path, key: &[u8; 32]) -> Result<(), String> {
    use base64::{engine::general_purpose::STANDARD, Engine};

    let encoded = Zeroizing::new(STANDARD.encode(key));
    entry()?
        .set_password(&encoded)
        .map_err(|e| format!("Couldn't save the biometric key in the OS keyring: {e}"))
}

#[cfg(target_os = "android")]
pub fn store(dir: &Path, key: &[u8; 32]) -> Result<(), String> {
    use std::io::Write;

    let mut temp = tempfile::NamedTempFile::new_in(dir)
        .map_err(|e| format!("Couldn't create biometric key file: {e}"))?;
    temp.write_all(key)
        .map_err(|e| format!("Couldn't write biometric key file: {e}"))?;
    temp.as_file()
        .sync_all()
        .map_err(|e| format!("Couldn't flush biometric key file: {e}"))?;
    temp.persist(dir.join("quick.key"))
        .map_err(|e| format!("Couldn't save biometric key file: {}", e.error))?;
    Ok(())
}

#[cfg(any(windows, target_os = "ios"))]
pub fn load(_dir: &Path) -> Result<Zeroizing<[u8; 32]>, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};

    let encoded = Zeroizing::new(
        entry()?
            .get_password()
            .map_err(|e| format!("Couldn't read the biometric key from the OS keyring: {e}"))?,
    );
    let bytes = Zeroizing::new(
        STANDARD
            .decode(encoded.as_bytes())
            .map_err(|_| "The saved biometric key is invalid".to_string())?,
    );
    let mut key: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| "The saved biometric key is invalid".to_string())?;
    let protected_key = Zeroizing::new(key);
    key.zeroize();
    Ok(protected_key)
}

#[cfg(target_os = "android")]
pub fn load(dir: &Path) -> Result<Zeroizing<[u8; 32]>, String> {
    let bytes = Zeroizing::new(
        std::fs::read(dir.join("quick.key"))
            .map_err(|e| format!("Couldn't read the saved biometric key: {e}"))?,
    );
    let mut key: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| "The saved biometric key is invalid".to_string())?;
    let protected_key = Zeroizing::new(key);
    key.zeroize();
    Ok(protected_key)
}

#[cfg(any(windows, target_os = "ios"))]
pub fn exists(_dir: &Path) -> Result<bool, String> {
    match entry()?.get_password() {
        Ok(password) => {
            drop(Zeroizing::new(password));
            Ok(true)
        }
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(error) => Err(format!("Couldn't check the OS keyring: {error}")),
    }
}

#[cfg(target_os = "android")]
pub fn exists(dir: &Path) -> Result<bool, String> {
    Ok(dir.join("quick.key").exists())
}

#[cfg(any(windows, target_os = "ios"))]
pub fn remove(_dir: &Path) -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(format!(
            "Couldn't remove the biometric key from the OS keyring: {error}"
        )),
    }
}

#[cfg(target_os = "android")]
pub fn remove(dir: &Path) -> Result<(), String> {
    match std::fs::remove_file(dir.join("quick.key")) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Couldn't disable biometric unlock: {error}")),
    }
}

#[cfg(not(any(windows, target_os = "android", target_os = "ios")))]
pub fn store(_dir: &Path, _key: &[u8; 32]) -> Result<(), String> {
    Err("Biometric unlock is not supported on this platform".into())
}

#[cfg(not(any(windows, target_os = "android", target_os = "ios")))]
pub fn load(_dir: &Path) -> Result<Zeroizing<[u8; 32]>, String> {
    Err("Biometric unlock is not supported on this platform".into())
}

#[cfg(not(any(windows, target_os = "android", target_os = "ios")))]
pub fn exists(_dir: &Path) -> Result<bool, String> {
    Ok(false)
}

#[cfg(not(any(windows, target_os = "android", target_os = "ios")))]
pub fn remove(_dir: &Path) -> Result<(), String> {
    Err("Biometric unlock is not supported on this platform".into())
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn windows_credential_store_round_trips_a_secret() {
        let service = format!("com.daylight.diary.test.{}", uuid::Uuid::new_v4());
        let entry = keyring::Entry::new(&service, "vault-key").unwrap();
        entry.set_password("test-secret").unwrap();
        let stored = entry.get_password();
        let removed = entry.delete_credential();

        assert_eq!(stored.unwrap(), "test-secret");
        assert!(removed.is_ok());
    }
}
