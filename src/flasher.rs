//! Device mutation is unavailable until original artwork and recovery can be verified.
//!
//! Stock AFC exposes Media, not the protected Wallet destination. No caller-supplied
//! file, hash, acknowledgement or successful sync supplies the missing proof.
use crate::safety;
use anyhow::Result;
use std::path::Path;

pub fn apply<L: FnMut(&str)>(
    _udid: &str,
    _card: &str,
    _png: &[u8],
    _png2x: &[u8],
    _pdf: &[u8],
    _log: L,
) -> Result<()> {
    // First and only operation: no session, Media staging, Books sync or cleanup.
    safety::require_verified_recovery().map_err(anyhow::Error::msg)
}

pub fn restore<L: FnMut(&str)>(_udid: &str, _card: &str, _backup: &Path, _log: L) -> Result<()> {
    safety::require_verified_recovery().map_err(anyhow::Error::msg)
}
#[cfg(test)]
mod recovery_gate_tests {
    use super::*;

    #[test]
    fn apply_blocks_for_recovery_before_validation_or_any_device_access() {
        // Invalid card guarantees the pre-revision implementation cannot open a device.
        let mut logged = false;
        let error = apply("offline-test-device", "../invalid", b"", b"", b"", |_| {
            logged = true
        })
        .unwrap_err()
        .to_string();
        assert!(error.contains("recuperación"), "{error}");
        assert!(!logged);
    }

    #[test]
    fn restore_blocks_for_recovery_before_reading_any_local_backup() {
        let mut logged = false;
        let error = restore(
            "offline-test-device",
            "../invalid",
            Path::new("missing-test-backup"),
            |_| logged = true,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("recuperación"), "{error}");
        assert!(!logged);
    }
}
