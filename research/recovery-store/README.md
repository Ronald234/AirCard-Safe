# Recovery storage laboratory — not a device backup feature

This isolated Rust crate preserves three **supplied local byte streams**, flushes and reads each file back, and writes an integrity manifest last. It never connects to an iPhone, acquires protected files, restores artwork, changes caches or enables the application's writer. It is not linked into AirCard-Safe.exe.

`capture(new_directory, pseudonymous_binding, [slot0, slot1, slot2])` creates a new directory and refuses existing destinations. Explicit `None` means a caller-reported absent variant; it is not synthesized. `inspect` checks the three fixed slots, binding, size limits and SHA-256 digests after reopening. Neither API proves that supplied data was read from a device. A malicious author can forge both data and manifest. A binding digest is pseudonymous, not anonymous or authenticated.

Each file is opened with create-new semantics, flushed and read back before the next step. Errors retain partial files. There is no automatic cleanup or delete-before-rename. A missing/incomplete/changed manifest is refused; a directory from an interrupted capture cannot be silently reused. Unknown manifest fields cannot assert recovery authorization.

Limits: 32 MiB per slot, three slots, 4096-byte manifest, bounded reads. No raw UDID, card ID, issuer name or log text is stored in metadata. The supplied bytes themselves may be sensitive; do not use real phone data in this laboratory.

Unix captures request directory mode 0700 and file mode 0600. Final-component symlinks are refused when opening. Windows rejects detected reparse points, but private Windows ACL provisioning and adversarial parent-directory races are not resolved. Files are flushed via sync_all; Windows directory-metadata persistence, storage-controller behavior and real power-loss recovery are not certified. The laboratory remains unsuitable for live acquisition until these gaps are resolved.

Tests inject an error after the first flushed file, reopen complete snapshots, reject tampering/foreign binding/unknown fields/untracked data and verify existing data is not overwritten. These are local storage tests, not physical iPhone or power-cut tests.

Run: `cargo test --locked --manifest-path research/recovery-store/Cargo.toml`.
