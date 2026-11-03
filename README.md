# AirCard Safe

Experimental native Windows application for customizing Apple Wallet artwork.
Independent MIT-licensed modification of [Lumid-Off/AirCard-Windows](https://github.com/Lumid-Off/AirCard-Windows).
**No physical iPhone test has been performed.**

**Apple Card is not supported by the original static-artwork method.** Its digital face uses dynamic vector rendering, as stated in the [original Mac project's pinned README](https://github.com/Mak5er/AirCard/blob/52ba05dca4bdc590672c19d554701f4438e20338/README.md). This revision does not claim to fix Apple Card. Explicit Apple Card log context is classified read-only; opaque identifiers do not reliably reveal issuer/type, so the UI also requires the user to confirm the selected card is not Apple Card or an NFC membership credential.

**Original artwork and refresh sequence restored:** applying sends the three artwork files, then the selected card's `.cache` and `.pkcache` invalidation batches, using upstream's StreamingZip/AirTraffic method. Cache invalidation overwrites six protected cache files with upstream's invalid-data marker. **No verified Wallet backup or guaranteed restoration is available.** User acknowledgement is required. Transfer completion does not prove a visible change. See [method and remaining limits](docs/ORIGINAL-METHOD.md) and [refresh correction](docs/REFRESH-FIX.md).

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
3. Click **Buscar iPhone por USB**, then **Detectar tarjetas y pases**, and open the intended card in Wallet. Each fresh scan clears previous observations.
4. Stop detection, choose a PNG/JPG/WebP image, and adjust the crop.
5. Confirm that the selection is a supported static-artwork payment card, not Apple Card or a membership/NFC credential, and accept that there is no verified Wallet backup or guaranteed restoration.
6. Click **Aplicar imagen y refrescar Wallet**. Keep the phone connected through all three phases, Books restoration and cleanup. Do not repeat a failed write blindly.
7. Force-close Wallet through the iPhone's app switcher and reopen it to inspect the visual result. The program cannot verify that result.

Only if you already have a compatible complete artwork backup: detect the same card, select its existing backup folder and choose **Enviar imagenes del respaldo existente**. Local digest/binding checks are performed; protected destination bytes and the visual restoration remain unverified. This build does not create that backup.

Discovery now uses native unified logs, with a legacy USB fallback if initialization fails. It can show structured pass/NFC observations that the earlier syslog-only scanner missed. Untyped and incompatible passes are read-only; detecting one does not prove its artwork is compatible. Explicit membership/layout evidence revokes inferred payment compatibility for that identifier. Privacy-redacted identifiers cannot be recovered. No gym-pass compatibility claim is made. See [scanner source and upstream notice](docs/UNIFIED_SCANNER.md).

## Safety limits

- Wallet-only UI; password keypad support removed from the build and source tree.
- Exactly three artwork filenames and six fixed cache leaves are allowed, all scoped to the selected payment-card identifier. Pass metadata, NFC credentials and keypad files are not write targets.
- Device/card identifiers remain in session memory. No card list, syslog or bank name is saved automatically.
- Backups contain artwork, SHA-256 digests and a pseudonymous device/card binding. Protect these files.
- Scanning stops after two minutes. Images are limited to 20 MiB, 8192 pixels per dimension and a 128 MiB decoding allocation limit.
- Artwork is generated at separate @3x (1536x969) and @2x (1024x646) resolutions; the earlier build reused the @3x bytes for both filenames.
- Ordinary AFC is used for Media staging, not for reading protected Wallet artwork. All staged payload bytes and the staged Books manifest are read back before dispatch.
- **No verified Wallet backup is created. Applying without one is the original transfer behavior and carries an explicit risk of changes that cannot be restored.**
- Captured artwork is the current appearance; it is not guaranteed to be the factory original.
- Incomplete, changed or wrong-device backups are rejected. Existing backup files are never silently overwritten.
- Tracked Books files are restored and read back after an attempted transfer. Protected Wallet destination bytes cannot be verified through ordinary AFC.
- Writes are not atomic. Disconnect, process termination or power loss can leave partial changes.
- Tracked Books state is held in memory and does not survive process termination. No crash-safe rollback is guaranteed.
- Recovery runs after write errors and combines write/restore/cleanup failures. A failed phase stops subsequent phases; no automatic file-by-file retries or recursive symlink cleanup.
- Refresh deliberately overwrites `FrontFace`, `PlaceHolder`, and `Preview` inside the selected card's `.cache` and `.pkcache` directories using the upstream `corrupted` marker. This is not a verified deletion or guaranteed cache regeneration. Its omission in the earlier Safe build could prevent a visible refresh; restoring it still needs physical validation.
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

