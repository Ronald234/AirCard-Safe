use crate::{afc::AfcClient, crypto, safety};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

#[derive(Serialize, Deserialize)]
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
fn save_new(path: &Path, data: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .context(
            "Backup file already exists or folder is not writable; select a new empty folder",
        )?;
    file.write_all(data)?;
    file.sync_all()?;
    safety::verify_bytes(data, &fs::read(path)?).map_err(anyhow::Error::msg)
}
pub fn load_original_assets(root: &Path, udid: &str, card: &str) -> Result<Vec<(String, Vec<u8>)>> {
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
pub fn capture_original_card(
    afc: &AfcClient,
    root: &Path,
    udid: &str,
    card: &str,
) -> Result<String> {
    ensure!(
        safety::valid_card_component(card),
        "Invalid card identifier"
    );
    let raw = card.trim_end_matches('=');
    let variants = [card.to_string(), raw.to_string(), format!("{raw}=")];
    let resolved = variants.iter().find(|v| afc.exists(&format!("/var/mobile/Library/Passes/Cards/{v}.pkpass")))
        .context("Wallet artwork is not readable over AFC. No backup can be verified; writing is blocked.")?.to_string();
    let dir = format!("/var/mobile/Library/Passes/Cards/{resolved}.pkpass");
    if root.join("backup.json").is_file() {
        let assets = load_original_assets(root, udid, &resolved)?;
        // Do not mix an old backup with a different current card state.
        for (leaf, expected) in assets {
            let actual = afc.read_file(&format!("{dir}/{leaf}"))?;
            safety::verify_bytes(&expected, &actual).map_err(anyhow::Error::msg)
                .context("Current artwork differs from this backup. Restore it first, or capture current artwork to a new empty folder.")?;
        }
        return Ok(resolved);
    }
    fs::create_dir_all(root)?;
    if fs::read_dir(root)?.next().is_some() {
        bail!(
            "Select an empty backup folder. Incomplete backups are never overwritten automatically."
        );
    }
    let mut digests = Vec::new();
    // Read all original files twice before any local or device modification.
    let mut originals = Vec::new();
    for leaf in safety::ARTWORK {
        let path = format!("{dir}/{leaf}");
        let bytes = afc
            .read_file(&path)
            .context("Could not read all three original artwork files; write blocked")?;
        validate_artwork(leaf, &bytes)?;
        safety::verify_bytes(&bytes, &afc.read_file(&path)?).map_err(anyhow::Error::msg)?;
        originals.push((leaf, bytes));
    }
    for (leaf, bytes) in originals {
        save_new(&root.join(leaf), &bytes)?;
        digests.push(crypto::sha256(&bytes)?);
    }
    let manifest = Manifest {
        version: 1,
        binding: binding(udid, &resolved)?,
        digests,
    };
    // Commit marker written last; interrupted backups cannot be loaded as complete.
    save_new(
        &root.join("backup.json"),
        &serde_json::to_vec_pretty(&manifest)?,
    )?;
    load_original_assets(root, udid, &resolved)?;
    Ok(resolved)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backup_rejects_incomplete_tampered_and_other_device() {
        let dir = std::env::temp_dir().join(format!("aircard-test-{}", crypto::token().unwrap()));
        fs::create_dir(&dir).unwrap();
        let card = "OM6NYhwXMZrAw0sRUjR62wmF4ZQ=";
        assert!(load_original_assets(&dir, "phone", card).is_err());
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
        assert_eq!(load_original_assets(&dir, "phone", card).unwrap().len(), 3);
        assert!(load_original_assets(&dir, "other-phone", card).is_err());
        fs::write(dir.join(safety::ARTWORK[0]), b"changed").unwrap();
        assert!(load_original_assets(&dir, "phone", card).is_err());
        // Generated test fixture folder, no user input.
        fs::remove_dir_all(dir).unwrap();
    }
}
