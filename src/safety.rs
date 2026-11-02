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
            "Operation incomplete; partial changes are possible. {}. Keep the backup and do not retry blindly.",
            errors.join("; ")
        ))
    }
}
