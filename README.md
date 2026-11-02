# AirCard Safe

Experimental native Windows application for customizing Apple Wallet artwork.
Independent MIT-licensed modification of [Lumid-Off/AirCard-Windows](https://github.com/Lumid-Off/AirCard-Windows).
**No physical iPhone test has been performed.**

## Download and run

Open this repository's Actions tab and select a successful **AirCard Safe Windows** run.
Download **AirCard-Safe-Windows-x64**, extract it, and open **AirCard-Safe.exe**.
GitHub may require sign-in to download an artifact. Artifacts expire after 30 days.

Compare `Get-FileHash .\AirCard-Safe.exe -Algorithm SHA256` with **SHA256SUMS.txt**.
The hash checks file consistency, not the publisher's identity; the executable is not code-signed.

No Python, Git, Rust, Cargo or Visual Studio is needed on the end user's computer.
Windows 10/11 x64 is the intended target. Apple Mobile Device Support and its USB driver must already be installed from official Apple software.
This application does not redistribute Apple's proprietary DLLs or install development tools.

## Use

1. Connect an unlocked iPhone by USB and trust the computer.
2. Disable Wi-Fi sync in Apple Devices/iTunes. Safe refuses a device with a concurrent network or unknown transport advertisement.
3. Click **Buscar iPhone por USB**, then **Detectar tarjeta**, and tap the card in Wallet.
4. Stop detection, choose a PNG/JPG/WebP image, and adjust the crop.
5. Select an empty backup folder. Applying is blocked unless all three current artwork files can be read and verified.
6. Acknowledge the non-atomic write and apply. Keep the phone connected.
7. Close and reopen Wallet to inspect the visual result.

To restore, detect the same card, select its existing verified backup folder, acknowledge the operation and choose **Restaurar imagenes del respaldo**.

## Safety limits

- Wallet-only UI; password keypad support removed from the build and source tree.
- Only three card artwork filenames are allowed by the writer.
- Device/card identifiers remain in session memory. No card list, syslog or bank name is saved automatically.
- Backups contain artwork, SHA-256 digests and a pseudonymous device/card binding. Protect these files.
- Scanning stops after two minutes. Images are limited to 20 MiB, 8192 pixels per dimension and a 128 MiB decoding allocation limit.
- All three original artwork files must be readable twice consistently, saved locally, read back and verified before applying.
- **Stock iOS may prevent reading Wallet artwork over AFC. Safe then refuses to write.** There is no unsafe bypass.
- Captured artwork is the current appearance; it is not guaranteed to be the factory original.
- Incomplete, changed or wrong-device backups are rejected. Existing backup files are never silently overwritten.
- Destination artwork and the tracked Books files are read back. This does not prove a visual refresh or preserve the entire Books application state.
- Writes are not atomic. Disconnect, process termination or power loss can leave partial changes.
- Tracked Books state is held in memory and does not survive process termination. No crash-safe rollback is guaranteed.
- Recovery runs after write errors and combines write/restore/cleanup failures. No automatic retries, recursive symlink cleanup or deliberate cache corruption.
- AirTrafficHost chooses transport by UDID. Safe rejects concurrent non-USB advertisements and rechecks before service/asset dispatch; physical transport isolation remains unverified.
- The private AirTrafficHost read API can block. Safe keeps the worker alive and prevents normal closure while it may still write. There is no guaranteed timeout.

## Build and validation

See the successful Actions run and its **build-info.json**, **cargo-test.log**, **cargo-build.log** and **TESTING.md**.
CI runs `cargo test --locked` and `cargo build --locked --release` on `windows-latest`.
The artifact must pass PE32+ AMD64 GUI header validation, a self-check and a brief GUI launch smoke test before upload.
A successful CI run is not a physical iPhone test or a Windows 10 hardware certification.

The package identity remains `aircard 1.2.4` to preserve upstream Cargo.lock; the binary target is **AirCard-Safe**.
Static CRT is requested in .cargo/config.toml. LICENSE is retained unchanged.
Pinned upstream revision: `88131e301c88fd9238cbeff9d091967c3ddbf10f`.
Credits: Lumid-Off (Windows), mak5er (original application), 0xjohnnydev/airlift (underlying research).
