# AirCard Safe - revision 3 build validation

## GitHub Actions delivery

Run: https://github.com/Ronald234/AirCard-Safe/actions/runs/41650863325
Built source: d54064a3c3bfde480be2d1e6afa03ed5506180cd
Artifact: https://github.com/Ronald234/AirCard-Safe/actions/runs/41650863325/artifacts/13236984039
Name: AirCard-Safe-Windows-x64
Executable SHA-256: 8113c57564313ef3cc003e8edd8dde2853385e813db8d4215551d9bf0efd84e9
ZIP digest from GitHub: 560c7a70d9ff644f90d134608736514b14e2459603999f25c2ebcd3c0776afa8
Artifact expires: 2026-12-03.

All Actions steps passed on windows-latest: cargo test --locked (50 passed, 0 failed, 1 physical-device test explicitly ignored), cargo build --locked --release, formatting, eight PE verifier fixtures, actual PE32+ AMD64 Windows GUI header validation, executable self-check, debug/release launch smoke tests and artifact publication.

Extract the artifact and open AirCard-Safe.exe. SHA256SUMS.txt, build-info.json, test/build logs, LICENSE, TESTING.md and the adapted scanner's upstream MIT notice are included. No Python, Git, Rust, Cargo or Visual Studio installation is required to run the executable. Official Apple USB support remains a prerequisite.

## Corrections

The original artwork -> .cache -> .pkcache sequence is restored with exact selected-card targets and fail-fast error handling. Separate @2x/@3x images are prepared. Native unified logs support structured pass observations without a Python helper. Observations merge by identifier, explicit incompatible evidence revokes inferred payment compatibility, and a full observation budget cannot suppress that revocation. Untyped/incompatible passes remain read-only.

See REFRESH-FIX.md, UNIFIED_SCANNER.md and TESTING.md for implementation, attribution and testing detail. Personal device metadata and private manual-test details are intentionally omitted from this public report.

## Compatibility and recovery limits

Apple Card's dynamic face is not supported by the original static-artwork method. Explicit Apple Card context is read-only and the UI requires compatibility acknowledgement. Opaque identifiers alone cannot reliably reveal issuer or pass type.

Gym/NFC discovery is not artwork compatibility. Untyped or incompatible pass observations remain read-only; privacy-redacted identifiers cannot be recovered. CI does not test real-device artwork changes, restoration, payments or gym access.

Protected destination bytes and cache regeneration cannot be verified through ordinary AFC. No verified Wallet backup or guaranteed restoration exists. Writes can be partial; tracked Books snapshots are in memory and do not survive process death. Private Apple calls can block despite software deadlines. USB selection checks are implemented but end-to-end physical route isolation is not certified. The executable is unsigned; its checksum verifies byte consistency, not publisher identity.

## Local delivery limitation

Automatic download/extraction of the CI archive into the local workspace was blocked by the environment's approval policy. No alternate automated download route was used. The already verified local revision remains available; users can download the CI artifact through the link above. Its own SHA256SUMS.txt is authoritative for the downloaded executable.
