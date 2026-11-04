//! Legacy local files can be inspected for integrity, never used as recovery proof.
use crate::{crypto, safety};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    // Pseudonymous linkage, no raw UDID, card hash, name or syslog.
    binding: String,
    digests: Vec<String>,
}
fn binding(udid: &str, card: &str) -> Result<String> {
    crypto::sha256(
        format!(
            "AirCard-Safe-v1\0{}\0{}",
            udid.to_ascii_lowercase(),
            card.trim_end_matches('=')
        )
        .as_bytes(),
    )
}
/// Refuse legacy local files as device originals. Hashes cannot establish provenance.
pub fn load_original_assets(
    _root: &Path,
    _udid: &str,
    _card: &str,
) -> Result<Vec<(String, Vec<u8>)>> {
    Err(anyhow::Error::msg(safety::RECOVERY_BLOCK_REASON))
}

/// Read-only integrity inspection; success is NOT original-artwork or recovery proof.
/// Never feed these bytes to a device writer or treat the result as authorization.
pub fn inspect_unverified_local_assets(
    root: &Path,
    udid: &str,
    card: &str,
) -> Result<Vec<(String, Vec<u8>)>> {
    ensure!(
        safety::valid_card_component(card),
        "Invalid card identifier"
    );
    ensure!(
        fs::metadata(root.join("backup.json"))?.len() < 4096,
        "Backup manifest too large"
    );
    let manifest: Manifest = serde_json::from_slice(&fs::read(root.join("backup.json"))?)?;
    ensure!(
        manifest.version == 1 && manifest.binding == binding(udid, card)?,
        "Backup belongs to another card/device or has an unsupported format"
    );
    ensure!(manifest.digests.len() == 3, "Backup is incomplete");
    let mut assets = Vec::new();
    for (index, leaf) in safety::ARTWORK.iter().enumerate() {
        let path = root.join(leaf);
        ensure!(
            fs::metadata(&path)?.len() <= 32 * 1024 * 1024,
            "Backup artwork too large"
        );
        let data = fs::read(path)?;
        ensure!(
            !data.is_empty() && crypto::sha256(&data)? == manifest.digests[index],
            "Backup artwork failed SHA-256 verification"
        );
        validate_artwork(leaf, &data)?;
        assets.push((leaf.to_string(), data));
    }
    Ok(assets)
}
fn validate_artwork(leaf: &str, bytes: &[u8]) -> Result<()> {
    if leaf.ends_with(".png") {
        ensure!(
            bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
            "Original artwork is not a PNG"
        );
    } else {
        ensure!(bytes.starts_with(b"%PDF-"), "Original artwork is not a PDF");
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backup_rejects_incomplete_tampered_and_other_device() {
        let dir = std::env::temp_dir().join(format!("aircard-test-{}", crypto::token().unwrap()));
        fs::create_dir(&dir).unwrap();
        let card = "AAAAAAAAAAAAAAAAAAAAAAAAAAA=";
        assert!(inspect_unverified_local_assets(&dir, "phone", card).is_err());
        let bytes: [&[u8]; 3] = [
            b"\x89PNG\r\n\x1a\nexample-a",
            b"\x89PNG\r\n\x1a\nexample-b",
            b"%PDF-example",
        ];
        for (leaf, data) in safety::ARTWORK.iter().zip(bytes) {
            fs::write(dir.join(leaf), data).unwrap();
        }
        let manifest = Manifest {
            version: 1,
            binding: binding("phone", card).unwrap(),
            digests: bytes.iter().map(|b| crypto::sha256(b).unwrap()).collect(),
        };
        fs::write(
            dir.join("backup.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert_eq!(
            inspect_unverified_local_assets(&dir, "phone", card)
                .unwrap()
                .len(),
            3
        );
        assert!(
            load_original_assets(&dir, "phone", card).is_err(),
            "Self-authored manifest cannot establish original artwork provenance or recovery"
        );
        assert!(inspect_unverified_local_assets(&dir, "other-phone", card).is_err());
        fs::write(dir.join(safety::ARTWORK[0]), b"changed").unwrap();
        assert!(inspect_unverified_local_assets(&dir, "phone", card).is_err());
        // Generated test fixture folder, no user input.
        fs::remove_dir_all(dir).unwrap();
    }
}
