// Pure policy functions: also compiled independently in integration tests.

pub fn restore_files(
    snapshot: &[(String, Option<Vec<u8>>)],
    mut read: impl FnMut(&str) -> Result<Option<Vec<u8>>, String>,
    mut write: impl FnMut(&str, &[u8]) -> Result<(), String>,
    mut remove: impl FnMut(&str) -> Result<(), String>,
) -> Result<(), String> {
    let mut errors = Vec::new();
    for (path, original) in snapshot {
        let outcome = (|| -> Result<(), String> {
            match original {
                Some(bytes) => {
                    write(path, bytes)?;
                    match read(path)? {
                        Some(actual) if &actual == bytes => Ok(()),
                        _ => Err("Restored bytes differ or file is missing".into()),
                    }
                }
                None => {
                    if read(path)?.is_some() {
                        remove(path)?;
                    }
                    match read(path)? {
                        None => Ok(()),
                        Some(_) => Err("New file remains after cleanup".into()),
                    }
                }
            }
        })();
        if let Err(error) = outcome {
            errors.push(format!("{path}: {error}"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

pub const ARTWORK: [&str; 3] = [
    "cardBackgroundCombined@3x.png",
    "cardBackgroundCombined@2x.png",
    "cardBackgroundCombined.pdf",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalletBatchKind {
    Artwork,
    Cache,
    PkCache,
}

pub const CACHE_LEAVES: [&str; 3] = ["FrontFace", "PlaceHolder", "Preview"];
// Exact upstream invalidation marker. This changes protected cache files and
// is not a deletion, a verified refresh, or a guaranteed reversible operation.
pub const CACHE_INVALIDATION_BYTES: &[u8] = b"corrupted";

pub fn wallet_cache_target(
    card: &str,
    kind: WalletBatchKind,
    items: &[(&str, &[u8])],
) -> Result<String, String> {
    let suffix = match kind {
        WalletBatchKind::Cache => ".cache",
        WalletBatchKind::PkCache => ".pkcache",
        WalletBatchKind::Artwork => return Err("Artwork is not a cache target".into()),
    };
    if !valid_card_component(card) || items.len() != CACHE_LEAVES.len() {
        return Err("Invalid Wallet cache target or incomplete batch".into());
    }
    for (index, (leaf, bytes)) in items.iter().enumerate() {
        if *leaf != CACHE_LEAVES[index] || *bytes != CACHE_INVALIDATION_BYTES {
            return Err("Wallet cache refresh is outside the fixed allowlist".into());
        }
    }
    Ok(format!("/var/mobile/Library/Passes/Cards/{card}{suffix}"))
}

pub fn run_wallet_sequence(
    mut submit: impl FnMut(WalletBatchKind) -> Result<(), String>,
) -> Result<(), String> {
    for kind in [
        WalletBatchKind::Artwork,
        WalletBatchKind::Cache,
        WalletBatchKind::PkCache,
    ] {
        submit(kind).map_err(|error| {
            format!("Wallet phase {kind:?} failed; partial changes are possible: {error}")
        })?;
    }
    Ok(())
}

pub fn wallet_batch_target(card: &str, items: &[(&str, &[u8])]) -> Result<String, String> {
    let target = format!("/var/mobile/Library/Passes/Cards/{card}.pkpass");
    if items.len() != ARTWORK.len() {
        return Err("All three Wallet artwork assets are required".into());
    }
    for (index, (leaf, bytes)) in items.iter().enumerate() {
        if !valid_target(&target, leaf) || *leaf != ARTWORK[index] {
            return Err("Write target is outside the Wallet artwork allowlist".into());
        }
        if bytes.is_empty() || bytes.len() > 32 * 1024 * 1024 {
            return Err("Invalid artwork payload size".into());
        }
        let signature: &[u8] = if leaf.ends_with(".png") {
            b"\x89PNG\r\n\x1a\n"
        } else {
            b"%PDF-"
        };
        if !bytes.starts_with(signature) {
            return Err("Artwork payload has the wrong file signature".into());
        }
    }
    Ok(target)
}

// Check Media staging bytes, never claim this proves protected destination bytes.
pub fn verify_staged_payloads(
    items: &[(&str, &[u8])],
    mut read: impl FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<(), String> {
    for (path, expected) in items {
        verify_bytes(expected, &read(path)?).map_err(str::to_string)?;
    }
    Ok(())
}

pub fn valid_card_component(value: &str) -> bool {
    let raw = value.trim_end_matches('=');
    let expected = match raw.len() {
        27 => 28,
        43 => 44,
        _ => return false,
    };
    (value.len() == raw.len() || value.len() == expected)
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'-' || b == b'_')
}

pub fn valid_target(dir: &str, leaf: &str) -> bool {
    let Some(card) = dir
        .strip_prefix("/var/mobile/Library/Passes/Cards/")
        .and_then(|v| v.strip_suffix(".pkpass"))
    else {
        return false;
    };
    valid_card_component(card) && ARTWORK.contains(&leaf)
}

pub fn verify_bytes(expected: &[u8], actual: &[u8]) -> Result<(), &'static str> {
    if expected.is_empty() || expected != actual {
        Err("File verification failed; operation stopped.")
    } else {
        Ok(())
    }
}

// AirTrafficHost chooses its own route by UDID. Fail closed on dual transport.
pub fn usb_route_allowed(usb: bool, network: bool) -> bool {
    usb && !network
}

// Always attempt both recovery phases and retain every failure.
pub fn run_with_cleanup(
    work: impl FnOnce() -> Result<(), String>,
    restore: impl FnOnce() -> Result<(), String>,
    cleanup: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    let write = work();
    let recovery = restore();
    let cleanup = cleanup();
    let mut errors = Vec::new();
    for (phase, outcome) in [
        ("Artwork", write),
        ("Books restore", recovery),
        ("Cleanup", cleanup),
    ] {
        if let Err(error) = outcome {
            errors.push(format!("{phase}: {error}"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Operation incomplete; partial changes are possible. {}. Preserve any existing recovery files and do not retry blindly.",
            errors.join("; ")
        ))
    }
}
