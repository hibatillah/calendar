use super::*;

pub(super) fn credential_entry(slot: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(CREDENTIAL_SERVICE, slot)
        .map_err(|_| "OS credential storage unavailable.".into())
}

pub(super) fn restore_credentials(slot: &str) -> Result<Option<Credentials>, String> {
    let current = credential_entry(slot)?;
    let value = match current.get_password() {
        Ok(value) => value,
        Err(keyring::Error::NoEntry) => {
            let legacy = keyring::Entry::new(LEGACY_CREDENTIAL_SERVICE, slot)
                .map_err(|_| "OS credential storage unavailable.")?;
            match legacy.get_password() {
                Ok(value) => {
                    let _: Credentials = serde_json::from_str(&value)
                        .map_err(|_| "Stored Google credentials are invalid.")?;
                    current
                        .set_password(&value)
                        .map_err(|_| "Cannot migrate Google credentials to Calendar.")?;
                    legacy
                        .delete_credential()
                        .map_err(|_| "Cannot remove the migrated credential entry.")?;
                    tracing::info!(action = "google_credentials_migrated");
                    value
                }
                Err(keyring::Error::NoEntry) => return Ok(None),
                Err(_) => return Err("Cannot read Google credentials from the OS vault.".into()),
            }
        }
        Err(_) => return Err("Cannot read Google credentials from the OS vault.".into()),
    };
    serde_json::from_str(&value)
        .map(Some)
        .map_err(|_| "Stored Google credentials are invalid. Disconnect and reconnect.".into())
}

pub(super) async fn store_credentials(
    slot: String,
    credentials: Option<Credentials>,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let entry = credential_entry(&slot)?;
        match credentials {
            Some(mut credentials) => {
                credentials.access_token.clear();
                credentials.expires_at = 0;
                entry
                    .set_password(
                        &serde_json::to_string(&credentials)
                            .map_err(|_| "Unable to encode credentials.")?,
                    )
                    .map_err(|_| "Unable to save credentials in the OS vault.".into())
            }
            None => {
                // Remove both namespaces so an interrupted migration cannot restore a signed-out account.
                for service in [CREDENTIAL_SERVICE, LEGACY_CREDENTIAL_SERVICE] {
                    let entry = keyring::Entry::new(service, &slot)
                        .map_err(|_| "OS credential storage unavailable.")?;
                    match entry.delete_credential() {
                        Ok(()) | Err(keyring::Error::NoEntry) => (),
                        Err(_) => {
                            return Err("Unable to remove credentials from the OS vault.".into());
                        }
                    }
                }
                Ok(())
            }
        }
    })
    .await
    .map_err(|_| "Credential storage task failed.")?
}
