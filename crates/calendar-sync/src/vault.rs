//! Refresh tokens in Windows Credential Manager: one generic credential per account.
//!
//! Target name `calendar.google.local/<account_id>`, user name `<account_id>`. The blob is JSON and
//! stays under the 2560-byte limit because profile data lives in the cache, not here. Access
//! tokens are never stored. This type has no `Debug`, so a secret cannot reach a log by accident.

use crate::Error;
use serde::{Deserialize, Serialize};
use std::{ffi::OsStr, os::windows::ffi::OsStrExt, ptr, slice};
use windows::{
    Win32::Security::Credentials::{
        CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredDeleteW, CredFree,
        CredReadW, CredWriteW,
    },
    core::{HRESULT, PCWSTR, PWSTR},
};

pub const SERVICE: &str = "calendar.google.local";
const ERROR_NOT_FOUND: u32 = 1168;

#[derive(Serialize, Deserialize)]
pub struct Secret {
    pub refresh_token: String,
    /// Space-separated scopes Google actually granted.
    pub scope: String,
}

pub struct Vault {
    service: String,
}

fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain([0]).collect()
}

impl Vault {
    /// Production code passes [`SERVICE`]; tests pass their own name so real entries are never touched.
    pub fn new(service: &str) -> Self {
        Self {
            service: service.to_owned(),
        }
    }

    fn target(&self, account_id: &str) -> Vec<u16> {
        wide(&format!("{}/{account_id}", self.service))
    }

    pub fn save(&self, account_id: &str, secret: &Secret) -> Result<(), Error> {
        let mut target = self.target(account_id);
        let mut user = wide(account_id);
        let mut blob = serde_json::to_vec(secret).map_err(|_| Error::Vault)?;
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: PWSTR(target.as_mut_ptr()),
            CredentialBlobSize: u32::try_from(blob.len()).map_err(|_| Error::Vault)?,
            CredentialBlob: blob.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            UserName: PWSTR(user.as_mut_ptr()),
            ..Default::default()
        };
        // SAFETY: every pointer in `credential` points into a local that outlives the call.
        let result = unsafe { CredWriteW(&credential, 0) };

        result.map_err(|_| Error::Vault)
    }

    /// `Ok(None)` when no entry exists; an unreadable or corrupt entry is an error.
    pub fn load(&self, account_id: &str) -> Result<Option<Secret>, Error> {
        let target = self.target(account_id);
        let mut found: *mut CREDENTIALW = ptr::null_mut();
        // SAFETY: `target` is NUL-terminated and `found` is a valid out pointer.
        let read =
            unsafe { CredReadW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None, &mut found) };
        if let Err(error) = read {
            return if error.code() == HRESULT::from_win32(ERROR_NOT_FOUND) {
                Ok(None)
            } else {
                Err(Error::Vault)
            };
        }

        // SAFETY: on success `found` points to a credential whose blob is `CredentialBlobSize`
        // bytes; the copy is parsed before the buffer is released.
        let secret = unsafe {
            let credential = &*found;
            let blob = if credential.CredentialBlob.is_null() {
                &[][..]
            } else {
                slice::from_raw_parts(
                    credential.CredentialBlob,
                    credential.CredentialBlobSize as usize,
                )
            };
            let secret = serde_json::from_slice(blob);
            CredFree(found.cast());

            secret
        };

        secret.map(Some).map_err(|_| Error::Vault)
    }

    /// A missing entry counts as deleted.
    pub fn delete(&self, account_id: &str) -> Result<(), Error> {
        let target = self.target(account_id);
        // SAFETY: `target` is NUL-terminated.
        let result = unsafe { CredDeleteW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None) };

        match result {
            Err(error) if error.code() != HRESULT::from_win32(ERROR_NOT_FOUND) => Err(Error::Vault),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_per_account_and_deletes_under_a_test_owned_service() {
        let vault = Vault::new(&format!("calendar.test.{}", uuid::Uuid::new_v4()));
        let secret = |token: &str| Secret {
            refresh_token: token.into(),
            scope: "scope".into(),
        };

        assert!(vault.load("one").unwrap().is_none());
        vault.save("one", &secret("token-one")).unwrap();
        vault.save("two", &secret("token-two")).unwrap();
        vault.save("one", &secret("token-one-again")).unwrap();

        assert_eq!(
            vault.load("one").unwrap().unwrap().refresh_token,
            "token-one-again"
        );
        assert_eq!(
            vault.load("two").unwrap().unwrap().refresh_token,
            "token-two"
        );
        vault.delete("one").unwrap();
        vault.delete("one").unwrap();
        vault.delete("two").unwrap();
        assert!(vault.load("one").unwrap().is_none());
        assert!(vault.load("two").unwrap().is_none());
    }
}
