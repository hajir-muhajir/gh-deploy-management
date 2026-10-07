//! Token storage backed by the OS credential store (Windows Credential Manager).
//!
//! The token never touches disk as plaintext and never leaves Rust — the webview
//! only ever sees the derived `TokenInfo`.

use keyring::{Entry, Error};

const SERVICE: &str = "gh-deploy-management";
const ACCOUNT: &str = "github-pat";

fn entry() -> Result<Entry, Error> {
    Entry::new(SERVICE, ACCOUNT)
}

pub fn store(token: &str) -> Result<(), Error> {
    entry()?.set_password(token)
}

/// `Ok(None)` when nothing has been stored yet, as opposed to a real failure.
pub fn read() -> Result<Option<String>, Error> {
    match entry()?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(Error::NoEntry) => Ok(None),
        Err(err) => Err(err),
    }
}

/// Idempotent: clearing an absent token is not an error.
pub fn clear() -> Result<(), Error> {
    match entry()?.delete_credential() {
        Ok(()) | Err(Error::NoEntry) => Ok(()),
        Err(err) => Err(err),
    }
}
