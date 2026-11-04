# AirCard Safe revision 4 — verified delivery

[Download the executable artifact](https://github.com/Ronald234/AirCard-Safe/actions/runs/41754963135/artifacts/13288007407) · [Successful Windows Actions run](https://github.com/Ronald234/AirCard-Safe/actions/runs/41754963135)

Built source commit: `1853eb1f327f588d090a550ee0f2c0e82a87ef6e`. The final delivery documentation is a later documentation-only commit; executable source is unchanged.

Download **AirCard-Safe-Windows-x64**, extract the ZIP and open **AirCard-Safe.exe**. No Python, Git, Rust, Cargo or Visual Studio is required by the executable. Official Apple USB support is required for phone discovery. GitHub may require sign-in. The artifact expires after 30 days.

SHA-256 of the **CI-built executable**:

```text
23c494c44658a5da6131a7d85cc22c974d13e878994c1de28b680b6d744cbab2  AirCard-Safe.exe
```

Compare against `SHA256SUMS.txt` inside the artifact. GitHub-reported artifact ZIP digest: `e8f03e926fceef8103226027d83e66a5b0f059a64d1d7083fe42c42057f650c4`. The ZIP was not independently downloaded and rehashed in this session; the executable header/hash and smoke checks were performed by the successful Windows runner before upload.

## Results actually observed

| Check | Result |
|---|---|
| Runner | windows-latest; complete job succeeded |
| cargo fmt --all -- --check | Passed |
| cargo test --locked | 117 passed executions, 0 failed, 2 ignored |
| Main unit suite | 56 passed, 1 ignored |
| Engine integration suite | 1 passed |
| Recovery integration suite | 7 passed |
| Safety integration suite | 15 passed |
| Scanner integration suite | 38 passed, 1 ignored |
| cargo clippy --locked --all-targets -- -D clippy::correctness | Passed; nonblocking dead-code/style warnings remain |
| cargo build --locked --release | Passed with ZIP 9.3.0 |
| PE verifier fixture suite | 8 passed |
| Actual CI executable | PE32+ AMD64 Windows GUI verification passed |
| Release --self-check | Passed |
| Debug and release GUI launch smoke | Passed, five seconds each without an iPhone |
| Source review | Three important findings corrected and re-reviewed; no remaining important/critical finding in that review |

Some unit tests repeat inside integration harnesses. The total is 117 successful executions, not 117 unique definitions. Physical USB enumeration repeats as ignored in two harnesses and is not a pass.

The executable artifact includes `build-info.json`, `SHA256SUMS.txt`, test/build/Clippy logs, dependency inventory, license and documentation. **AirCard-Safe-Diagnostic-Logs** is a separate CI-log artifact, not the application download and not a collection of phone logs.

## Current behavior and limitations

Apply and restore are blocked in both UI and backend before any device access. No trusted, non-destructive acquisition of the protected originals or verified recovery mechanism exists in this implementation. A local manifest, matching SHA-256 or acknowledgement cannot unlock writing. Earlier revision 3 write behavior is historical and does not meet this requirement.

Available: USB log discovery, conservative unknown/generic/loyalty/transit/access observations, aggregate-only diagnostic export, image crop/preparation and local PNG/PDF ZIP export. An identifier or cache path does not establish payment, image or NFC compatibility. No credentials, keys, banking data or authentication mechanisms are modified.

Physical iPhone detection, Apple Devices runtime loading, native cancellation timing, NFC/gym access, payment, protected-image restoration and physical Windows 10 validation remain unverified for this revision. See [compatibility](COMPATIBILITY.md), [recovery audit](RECOVERY-AUDIT.md), [transport audit](TRANSPORT-AUDIT.md) and [scanner diagnostics](SCANNER-DIAGNOSTICS.md).

A complete fresh RustSec dependency audit could not be verified complete. The supplementary cargo-audit attempt encountered a temporary-output access failure, then the local execution environment rejected an invalid Linux-style sandbox URI on Windows. Clippy, static review and targeted ZIP advisory review were completed; ZIP was upgraded from 2.4.2 to 9.3.0 and the full locked tests/build repeated. No clean-dependency or vulnerability-free claim is made. See [security review](SECURITY-REVIEW.md).
