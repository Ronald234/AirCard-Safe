# AirCard Safe - revision 3 validation

## Current local Windows result

- cargo test --locked: 50 passed, 0 failed, 1 explicitly ignored physical-iPhone test.
- cargo build --locked --release: passed.
- cargo fmt --all -- --check and git diff --check: passed.
- PE verifier fixtures: 8 passed.
- Actual executable: PE32+ AMD64 Windows GUI, 13,164,032 bytes.
- Actual release --self-check: passed.
- Release GUI launch smoke: passed (alive five seconds, no device operation invoked).
- Local OS reported: Microsoft Windows NT 10.0.26200.0.
- Local executable SHA-256: c2532e2570f4defaaba6189956d969a0c12dce0721073e78a6a2f9febfb4bbda.

The checksum above belongs to the local build. A GitHub Actions executable may have different bytes; use the SHA256SUMS.txt accompanying that artifact. The revision 3 Actions result will be linked after its run completes. No new Actions artifact is claimed by this source-status entry.

## Corrections

The original artwork -> .cache -> .pkcache sequence is restored with exact selected-card cache targets and fail-fast error handling. Distinct @2x/@3x images are prepared. Native unified logs support structured pass observations without a Python helper. Typed evidence is merged by identifier, incompatible evidence revokes inferred payment compatibility, and a full observation budget cannot suppress that revocation. Untyped/incompatible passes are read-only. See REFRESH-FIX.md, UNIFIED_SCANNER.md and TESTING.md.

## Limits that remain

Apple Card's dynamic face is not supported by the original static-artwork method. Its explicit log context is read-only and the UI requires compatibility acknowledgement. Opaque identifiers alone cannot reliably reveal issuer or pass type.

A gym/NFC pass observation is not proof of artwork compatibility. Such untyped or incompatible observations remain read-only. Privacy-redacted identifiers cannot be recovered.

No new physical artwork apply/restore, payment or gym-access test was performed. The user's earlier no-visible-change Apple Card report is recorded in DEVICE-FEEDBACK-2026-11-03.md; it is not successful device validation.

Protected Wallet destination bytes and cache regeneration cannot be verified through ordinary AFC. No verified Wallet backup or guaranteed restoration exists. Writes can be partial; tracked Books snapshots are in memory and do not survive process death. Native private Apple calls can block despite software deadlines. USB route checks are implemented but physical end-to-end route isolation is not certified. The executable is unsigned; a checksum proves byte consistency, not publisher identity.

## Historical build

The earlier strict-backup Windows build was verified by run https://github.com/Ronald234/AirCard-Safe/actions/runs/41551859987 with executable SHA-256 7056296e7302d68cc928b23cba0c0c2ade136ff7367aad79d49e12846dc5b430. It is not the revision 3 executable described above.
