# Historical local Windows validation - original transfer revision

This records the earlier build without the restored refresh sequence. Current results are in STATUS.md.

## Observed results

- `cargo test --locked`: passed, 27 tests passed, 0 failed, 1 explicitly ignored (requires physical iPhone and Apple USB drivers).
- `cargo build --locked --release`: passed.
- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.
- `scripts/test-verify-pe.ps1`: 8 fixtures passed.
- Actual release executable: PE32+, AMD64, Windows GUI, 13,122,048 bytes.
- Actual release executable `--self-check`: passed.
- Release GUI launch smoke: passed (process alive after five seconds; no device operation invoked).
- Debug executable `--self-check`: passed.
- OS version reported by the local runtime: Microsoft Windows NT 10.0.26200.0. This is not broad Windows 10/11 hardware certification.
- Release SHA-256: `83177b39518e4c09455a20bb5af30c1838763736e0bcd7decf457c53d82bcfb1`.

The build emitted unused-code warnings inherited from retained upstream modules; test/build commands returned zero. Cargo also fell back to copying incremental-cache files because hard linking was unavailable.

An additional diagnostic incorrectly ran the release-only PE verifier against the debug executable. It rejected the console subsystem, which is intentional in debug builds (`windows_subsystem = "windows"` applies only to release). The actual release passed that verifier; debug and release self-checks passed.

## Publication and device limits

The GitHub tree publication was rejected by automatic approval review: removing the verified-backup write gate enables potentially irreversible Wallet changes, and explicit authorization for that exact risk was required. No alternate publication route was used. The previously published executable remains unchanged.

Current source uses the original transfer mechanism with an explicit no-backup acknowledgement, bounded artwork targets and staged-byte checks; see ORIGINAL-METHOD.md. It does not provide protected destination readback or guaranteed restoration.

No physical iPhone operation was tested. The current revision has not yet completed a new GitHub Actions run, and no new Actions executable artifact is claimed here.
