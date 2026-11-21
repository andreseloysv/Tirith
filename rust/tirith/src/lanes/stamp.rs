//! Time stamps of lanes and runs as seconds since the epoch.

/// An RFC 3339 stamp in seconds; 0 when missing or unreadable.
pub(super) fn at_s(iso: Option<&str>) -> u64 {
    iso.and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map_or(0, |t| t.timestamp().max(0) as u64)
}
