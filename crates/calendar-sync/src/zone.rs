//! The display zone is the Windows system zone, recorded in the cache (`Store::zone`) so the
//! tray and the window always agree. Both resolve the system zone at every sync run, because it
//! can change while they run.

use crate::{Error, Store};
use chrono_tz::Tz;

/// The system zone as an IANA zone, or `None` when Windows or `chrono-tz` cannot name it.
pub fn system_zone() -> Option<Tz> {
    system_zone_name()?.parse().ok()
}

/// Windows answers with an IANA name from its own ICU data (`Windows.Globalization.Calendar`), so
/// there is no Windows-to-IANA table to keep current.
#[cfg(windows)]
fn system_zone_name() -> Option<String> {
    use windows::Globalization::Calendar;

    let calendar = Calendar::new().ok()?;

    calendar.GetTimeZone().ok().map(|name| name.to_string())
}

#[cfg(not(windows))]
fn system_zone_name() -> Option<String> {
    None
}

/// Makes the cache use `system`, and returns the zone to sync and display with. A different zone
/// drops every sync token (see `Store::record_zone`). When `system` is unknown the last recorded
/// zone stays in force, else UTC, and nothing is recorded.
pub fn align_cache_zone(store: &mut Store, system: Option<Tz>) -> Result<Tz, Error> {
    let recorded = store.zone()?;
    let Some(zone) = system else {
        tracing::warn!(
            action = "system_zone_unavailable",
            fallback = recorded.map_or("UTC", Tz::name)
        );

        return Ok(recorded.unwrap_or(Tz::UTC));
    };
    if store.record_zone(zone)? {
        tracing::info!(
            action = "cache_zone_changed",
            from = recorded.map_or("none", Tz::name),
            to = zone.name()
        );
    }

    Ok(zone)
}

#[cfg(test)]
mod tests;
