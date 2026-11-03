# AirCard Safe - verified Windows build
Built source commit: 19aa77fdcdcf6c54b45336c36f04e1db32bc114d
Windows Actions run: https://github.com/Ronald234/AirCard-Safe/actions/runs/41551859987
Executable SHA-256: 7056296e7302d68cc928b23cba0c0c2ade136ff7367aad79d49e12846dc5b430
Artifact: https://github.com/Ronald234/AirCard-Safe/actions/runs/41551859987/artifacts/13191908363

## Observed results
- cargo test --locked: 24 passed, 0 failed; 1 physical-iPhone test explicitly ignored.
- cargo build --locked --release: passed.
- cargo fmt --all -- --check: passed.
- Windows renderer includes a native DirectX 12 backend: regression test passed.
- Debug GUI startup: passed.
- Actual release executable: PE32+ AMD64 Windows GUI header verified.
- Actual release executable --self-check: passed.
- Release GUI launch smoke test: passed (five-second process-alive check, no device attached).
- PE verifier malformed/truncated header fixtures: 8 passed.
- MIT upstream license retained; code and test workflows publicly available.
- Password keypad module/UI removed. USB restrictions and explicit artwork backup checks remain enabled.

## Corrections verified by this build
Original OpenGL renderer failed on the hosted Windows runner because OpenGL 2+ was unavailable.
Safe now explicitly uses WGPU and enables its native backend defaults.
A regression test checks native DirectX 12 backend availability.
Cargo.lock was resolved on Windows, committed, and all final tests/builds then ran with --locked.
The temporary lockfile resolver workflow was removed.

## Required limits and pending physical checks
No physical iPhone test was performed. No claim is made that artwork customization works on a real phone.
Windows Server 2025 hosted CI is not a Windows 10/11 hardware certification.
Apple Mobile Device Support and USB drivers must already be installed from official Apple software.
Confirmed architectural blocker: the current client opens ordinary com.apple.afc (Media root), but backup and destination verification try to read protected Wallet artwork outside that root. This is not a working backup/readback route on stock iOS. Safe blocks applying before artwork writes. See ACCESS-REVIEW.md; Windows build validation is not evidence of functional customization.
The artwork backup is not a full-device backup, and captured current artwork is not necessarily factory-original.
Writes and restore are not atomic; unplugging, terminating the process or power loss can leave partial changes.
Tracked Books snapshots are in memory only. They do not preserve all Books state or survive a crash.
The private AirTrafficHost read API can block. There is no guaranteed timeout or rollback.
Only USB sessions are selected, and concurrent network/unknown advertisements are rejected. AirTrafficHost chooses its route by UDID; physical transport isolation remains unverified.
The executable is unsigned. The SHA-256 verifies byte consistency, not publisher identity.
See TESTING.md for the physical-device test checklist.

## Delivery
Download the named GitHub Actions artifact, extract it and open AirCard-Safe.exe.
No Python, Git, Rust, Cargo or Visual Studio is needed by the end user.
Artifacts expire after 30 days. SHA256SUMS.txt and build-info.json accompany the executable.
