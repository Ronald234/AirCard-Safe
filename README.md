# AirCard Safe - experimental development checkpoint

**No final AirCard-Safe.exe has been built or published in this session.**
This source checkpoint is not the requested finished executable, an installer, or a generator for your PC.
The executable must be produced and pass the GitHub Windows workflow before distribution.

Native Rust fork work based on [Lumid-Off/AirCard-Windows](https://github.com/Lumid-Off/AirCard-Windows), pinned upstream revision `88131e301c88fd9238cbeff9d091967c3ddbf10f`.
MIT license retained verbatim in LICENSE. Credits: Lumid-Off (Windows), mak5er (original app), 0xjohnnydev/airlift (underlying research).
This is an independent modification, not an official upstream release.

## Scope

- Wallet PNG/PDF artwork preparation, session-only card detection and explicit artwork restore.
- Password keypad theme modules/UI removed. Only three Wallet artwork filenames are allowed by the writer.
- USB sessions only. Network and unknown transports are rejected before writing.
- AirTrafficHost takes a UDID and chooses its own transport. Safe rejects any concurrent non-USB advertisement and rechecks before each dispatched asset. This is fail-closed routing policy, **not physical proof of transport isolation**.
- No card list, UDID, bank names, syslog files or settings are saved automatically. Device/card identifiers are held in memory during the session. The user-selected backup stores images, SHA-256 digests and a one-way device/card binding. This is pseudonymous data, not anonymous data; protect the backup.
- Syslog scanning stops after two minutes. No raw lines or detected names are displayed or persisted.
- Images are limited to 20 MiB, 8192 pixels per dimension and a 128 MiB decoder allocation limit.

## Backups and writes

Applying requires all three current artwork files to be readable over AFC, read twice consistently, saved locally, read back and SHA-256 verified. If the normal iPhone sandbox prevents access to Wallet artwork, **the application refuses to write**. There is no unsafe bypass. This stricter policy may prevent use on stock devices even where upstream flashing worked.
Captured artwork is the current appearance; it is not guaranteed to be the factory original.

A manifest is committed last. Incomplete or modified backups and backups for another device/card are rejected.
A backup never overwrites existing files automatically. Restore verifies identity and file integrity before writing.
Successful restoration means destination bytes were read back, not that Wallet visually refreshed or that a complete iPhone backup exists.

Writes are not atomic. A disconnect, process termination or power failure can leave partial changes.
Only tracked Books files are snapshotted in memory and restored with read-back. This does not capture the entire Books application state, and the snapshot does not survive process termination.
Recovery is attempted even on errors; write, restore and cleanup failures are combined.
No blind retries, recursive symlink cleanup or deliberate Wallet cache corruption.
Close/reopen Wallet manually. Cached display might require additional action; Safe does not damage caches to force a refresh.

Apple's private AirTrafficHost read API can block. Safe keeps the operation worker alive and prevents normal window closure while it may still write. There is no claim of a guaranteed timeout or crash-safe rollback.

## Download the finished executable (after a successful build exists)

1. Open the repository's **Actions** tab.
2. Open a successful **AirCard Safe Windows** run for the reviewed commit.
3. Download artifact **AirCard-Safe-Windows-x64** (GitHub sign-in may be required).
4. Extract it, compare `Get-FileHash .\AirCard-Safe.exe -Algorithm SHA256` with `SHA256SUMS.txt`.
5. Run `AirCard-Safe.exe`. No Python, Rust, Cargo, Git or Visual Studio is needed.

**There is currently no published repository/run/artifact URL for this checkpoint.**
Artifacts expire after 30 days. A SHA-256 file verifies consistency, not the publisher's identity; no code-signing certificate is configured.

Windows 10/11 x64 is the intended target; only a successful CI run will validate its runner.
Apple Mobile Device Support, including CoreFoundation.dll, MobileDevice.dll, AirTrafficHost.dll and its USB driver, must already be installed using official Apple software. These proprietary drivers/libraries are not redistributed here.
Do not download replacement DLLs from random sites. Safe does not install dependencies on the user's PC.

## Maintainer build

The workflow uses `windows-latest`, a Rust toolchain, `cargo test --locked`, and `cargo build --locked --release`.
The package identity remains `aircard 1.2.4` solely to preserve the original Cargo.lock; the binary target is `AirCard-Safe`.
Dependencies and Cargo.lock are unchanged. Static CRT is requested in .cargo/config.toml.
PE32+ AMD64 GUI header checks, a built-in self-check and GUI launch smoke test must pass before upload.
A workflow file existing is not evidence that it ran.

See [docs/TESTING.md](docs/TESTING.md) and [docs/STATUS.md](docs/STATUS.md) for actual evidence and remaining blockers.

