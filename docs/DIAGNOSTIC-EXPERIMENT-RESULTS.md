# Diagnostic executable experiment — verified results

Source commit: 5138ec5de222b90c1a8fb1f13e7bd51856aeb816.

Windows CI: https://github.com/Ronald234/AirCard-Safe/actions/runs/42122663645
Executable artifact: https://github.com/Ronald234/AirCard-Safe/actions/runs/42122663645/artifacts/13459473158

All Windows workflow steps succeeded: cargo test --locked (121 passed executions, 2 physical tests ignored), static correctness analysis, debug GUI startup, cargo build --locked --release, eight PE verifier regression fixtures, verification of the actual Windows binary, --self-check and release GUI startup. These are PC tests, not physical iPhone validation.

AirCard-Safe.exe SHA-256:
32be7e4574fe8364662fb4166a5abd302fc2d7332f446fd5326be8b3dec9f8db

The ZIP artifact digest is different from the executable digest. The artifact includes SHA256SUMS.txt and build-info.json.

The separate research workflow also succeeded:
https://github.com/Ronald234/AirCard-Safe/actions/runs/42122663592

The isolated diagnostic regression was observed failing before implementation, then all four diagnostic-schema tests passed locally. Local full-app tests were blocked by missing offline dependencies; the complete suite passed on the Windows runner. Independent review found no critical or important issue in the aggregate diagnostic change.

Download the executable artifact, extract it and run AirCard-Safe.exe. Start a fresh USB scan, open the intended pass directly in Wallet, stop and export a new diagnostic JSON. Confirm it contains unmatched_events. No development tools are required on the user's PC; Apple device communication support remains required for USB access.

This executable adds diagnostic evidence only. Device writes remain blocked; the payment_candidate heuristic is not a confirmed payment classification. No AF/NFC artwork compatibility or verified recovery has been established. Do not infer that a successful scan permits editing.
