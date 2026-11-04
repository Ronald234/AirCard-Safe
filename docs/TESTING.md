# Testing — revision 4

## Automated checks

Run in an appropriate Windows Rust development environment (not needed by the user of the executable):

```
cargo fmt --all -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D clippy::correctness
cargo build --locked --release
./scripts/test-verify-pe.ps1
./scripts/verify-pe.ps1 -Path target/release/AirCard-Safe.exe
```

Run the actual built release executable with `--self-check`; success exit code is 0. Then test a short GUI launch without a phone. CI automates both before artifact upload and records the results in build-info.json. It does not exercise private phone services.

Coverage includes:

- Backend apply/restore invoked with valid-looking synthetic IDs before any device access. The recovery integration crate excludes all device/transport modules, proving no transport side effect is reachable.
- Complete matching local manifest still cannot authorize restoration. Wrong-device/card, partial, tampered and unsupported manifests are rejected; backup files remain intact.
- Fixed image filename/batch constraints, first failure stops subsequent phases, combined cleanup/restore failures; these archival writer policies do not enable the release writer.
- USB-only route policy with ambiguous network rejection.
- Fragmented/truncated/malformed native log frames; record length bounds, cancellation and opaque receive errors.
- Additional Dashboard/selection events, read-only explicit pass types, unknown/redacted/no-ID observations, bounded inventory and monotonic evidence handling.
- Aggregate diagnostics export with secret sentinels and count saturation. No raw lines, names, paths, device metadata or opaque IDs can enter the JSON schema.
- Image decoder constraints, distinct @2x/@3x dimensions, crop behavior, local fixed-name ZIP export and refusal to overwrite an existing file.
- Protected Apple Store runtime path validation and no release AirTrafficHost dependency.
- PE verifier rejects malformed/non-Windows/wrong-machine/DLL fixtures.

Some tests share source modules through integration harnesses and therefore repeat unit tests; report distinct test suites separately when giving totals. One physical USB enumeration test is intentionally ignored by default. Do not count it as a pass.

## Manual checks pending

- Actual iPhone USB discovery through Apple Devices and standalone Apple Mobile Device Support.
- Native unified/legacy log availability, cancellation and private ABI behavior across iOS versions.
- A Wallet pass of each type: payment, generic, loyalty, transit and access, including events containing private/redacted IDs.
- Windows 10 and Windows 11 physical machines, scaling and graphics-driver variations.
- Any image modification, payment, gym entry or restoration: **not implemented as an enabled flow in this revision**. Establish a non-destructive, independently verified recovery mechanism first.

No developer or CI test here is a physical iPhone validation. See COMPATIBILITY.md and SECURITY-REVIEW.md.
