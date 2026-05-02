use keyring::Entry;

use crate::error::{AppError, AppResult};

const SERVICE: &str = "com.blxie.guitar-sheet.github";
const USER: &str = "personal_access_token";

fn entry() -> AppResult<Entry> {
    Entry::new(SERVICE, USER).map_err(|e| AppError::Keyring(e.to_string()))
}

pub fn set_pat(token: &str) -> AppResult<()> {
    let e = entry()?;
    e.set_password(token)
        .map_err(|err| AppError::Keyring(err.to_string()))
}

pub fn get_pat() -> AppResult<Option<String>> {
    let e = entry()?;
    match e.get_password() {
        Ok(s) if !s.is_empty() => Ok(Some(s)),
        Ok(_) => Ok(None),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(AppError::Keyring(err.to_string())),
    }
}

pub fn clear_pat() -> AppResult<()> {
    let e = entry()?;
    match e.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(AppError::Keyring(err.to_string())),
    }
}

pub fn pat_configured() -> AppResult<bool> {
    Ok(get_pat()?.is_some())
}
