//! Research-only local snapshot storage. Never acquires files from a phone, never
//! restores them and never proves provenance or authorizes a device write.
pub mod pass_artwork;

use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::Path,
};

#[cfg(windows)]
mod windows_privacy;

const MAX_ASSET: usize = 32 * 1024 * 1024;
const MAX_MANIFEST: usize = 4096;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    length: usize,
    sha256: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: u32,
    binding: String,
    assets: [Option<Entry>; 3],
}
/// Integrity of supplied local bytes only. This is deliberately not a recovery permit.
pub struct LocalSnapshot {
    assets: [Option<Vec<u8>>; 3],
}
impl LocalSnapshot {
    pub fn asset(&self, index: usize) -> Option<&[u8]> {
        self.assets.get(index).and_then(Option::as_deref)
    }
}
fn digest(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
fn binding_valid(binding: &str) -> bool {
    binding.len() == 64 && binding.bytes().all(|b| b.is_ascii_hexdigit())
}
fn reject_link(metadata: &fs::Metadata) -> Result<()> {
    ensure!(!metadata.file_type().is_symlink(), "Symbolic link refused");
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        ensure!(
            metadata.file_attributes() & 0x400 == 0,
            "Reparse point refused"
        );
    }
    Ok(())
}
fn directories_only(path: &Path) -> Result<()> {
    for ancestor in path.ancestors() {
        if ancestor.as_os_str().is_empty() {
            continue;
        }
        let metadata = fs::symlink_metadata(ancestor)?;
        reject_link(&metadata)?;
        ensure!(metadata.is_dir(), "Expected an existing ordinary directory");
    }
    Ok(())
}
fn read_regular(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let before = fs::symlink_metadata(path)?;
    reject_link(&before)?;
    ensure!(
        before.is_file() && before.len() <= limit as u64,
        "Invalid file or size limit exceeded"
    );
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x0020_0000);
    }
    #[cfg(windows)]
    windows_privacy::verify(path, false)?;
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    reject_link(&metadata)?;
    ensure!(
        metadata.is_file() && metadata.len() <= limit as u64,
        "Invalid file or size limit exceeded"
    );
    let expected_len = metadata.len();
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= limit && bytes.len() as u64 == expected_len,
        "File changed or exceeded size limit"
    );
    Ok(bytes)
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    #[cfg(windows)]
    windows_privacy::verify(path, false)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    ensure!(
        read_regular(path, bytes.len())? == bytes,
        "Local readback differs"
    );
    Ok(())
}
/// Creates a new capture directory, writes/flushes each supplied asset, then writes
/// the manifest last. On any error all partial files are retained, never deleted.
/// sync_all is an OS flush request, not a certification of power-loss recovery.
pub fn capture(root: &Path, binding: &str, assets: [Option<&[u8]>; 3]) -> Result<LocalSnapshot> {
    capture_impl(root, binding, assets, |_| Ok(()))
}
fn capture_impl(
    root: &Path,
    binding: &str,
    assets: [Option<&[u8]>; 3],
    mut after_file: impl FnMut(usize) -> Result<()>,
) -> Result<LocalSnapshot> {
    ensure!(
        binding_valid(binding),
        "Expected pseudonymous binding digest"
    );
    ensure!(assets.iter().any(Option::is_some), "Empty capture refused");
    ensure!(
        assets.iter().flatten().all(|data| data.len() <= MAX_ASSET),
        "Asset too large"
    );
    let parent = root
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    directories_only(parent)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new().mode(0o700).create(root)?;
    }
    #[cfg(windows)]
    windows_privacy::create(root)?; // Existing directories are never reused or overwritten.
    directories_only(root)?;
    let mut entries: [Option<Entry>; 3] = [None, None, None];
    for (index, data) in assets.into_iter().enumerate() {
        if let Some(data) = data {
            write_new(&root.join(format!("original-{index}.bin")), data)?;
            entries[index] = Some(Entry {
                length: data.len(),
                sha256: digest(data),
            });
            after_file(index)?;
        }
    }
    let manifest = Manifest {
        schema: 1,
        binding: binding.into(),
        assets: entries,
    };
    let bytes = serde_json::to_vec(&manifest)?;
    ensure!(bytes.len() <= MAX_MANIFEST, "Manifest too large");
    write_new(&root.join("manifest.json"), &bytes)?;
    #[cfg(unix)]
    fs::File::open(root)?.sync_all()?;
    inspect(root, binding)
}
/// Inspects a local capture without changing or cleaning anything. Matching
/// self-authored hashes establish integrity, never device origin or recoverability.
pub fn inspect(root: &Path, binding: &str) -> Result<LocalSnapshot> {
    ensure!(binding_valid(binding), "Invalid binding digest");
    directories_only(root)?;
    #[cfg(windows)]
    windows_privacy::verify(root, true)?;
    let manifest: Manifest =
        serde_json::from_slice(&read_regular(&root.join("manifest.json"), MAX_MANIFEST)?)?;
    ensure!(
        manifest.schema == 1 && manifest.binding == binding,
        "Unknown schema or different binding"
    );
    ensure!(manifest.assets.iter().any(Option::is_some), "Empty capture");
    let mut assets: [Option<Vec<u8>>; 3] = [None, None, None];
    for (index, entry) in manifest.assets.into_iter().enumerate() {
        let path = root.join(format!("original-{index}.bin"));
        if let Some(entry) = entry {
            ensure!(entry.length <= MAX_ASSET, "Asset length exceeds limit");
            let bytes = read_regular(&path, MAX_ASSET)?;
            ensure!(
                bytes.len() == entry.length && digest(&bytes) == entry.sha256,
                "Original integrity mismatch"
            );
            assets[index] = Some(bytes);
        } else {
            match fs::symlink_metadata(path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                _ => bail!("Unexpected data in an explicitly absent slot"),
            }
        }
    }
    for entry in fs::read_dir(root)? {
        let name = entry?.file_name();
        ensure!(
            [
                "manifest.json",
                "original-0.bin",
                "original-1.bin",
                "original-2.bin"
            ]
            .iter()
            .any(|allowed| name == *allowed),
            "Unexpected capture file; retained for inspection"
        );
    }
    Ok(LocalSnapshot { assets })
}

#[cfg(test)]
mod interruption_tests {
    use super::*;
    #[test]
    fn failure_after_first_flushed_asset_retains_bytes_and_does_not_seal() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("capture");
        let result = capture_impl(
            &root,
            &"a".repeat(64),
            [Some(b"original"), Some(b"second"), None],
            |_| bail!("injected interruption"),
        );
        assert!(result.is_err(), "the fault must interrupt capture");
        assert_eq!(fs::read(root.join("original-0.bin")).unwrap(), b"original");
        assert!(!root.join("manifest.json").exists());
        assert!(inspect(&root, &"a".repeat(64)).is_err());
    }
}
