# AirCard Safe design
User request is the specification; autonomous execution is explicitly authorized.
Native Rust Windows x64 application derived from MIT upstream commit 88131e301c88fd9238cbeff9d091967c3ddbf10f.
Keep Wallet artwork preparation/detection only. Replace broad original UI with a minimal in-memory UI.
Remove keypad theme support entirely. USB-only device sessions, reject devices advertised on network before AirTraffic because that private API accepts only UDID.
Fail closed without a complete, byte-verified original artwork backup. Keep backups in user-chosen directories with no device/card identifiers in filenames. Match backup to connected card before writes.
No cache corruption, no fallback retries after partial writes, report combined write/cleanup failures. Do not call protocol completion physical verification.
Alternatives: retain original broad UI (rejected: unnecessary surface); rewrite device protocol (rejected: untested); reuse original protocol with narrow UI and safety gates (selected engineering default).
No physical iPhone validation or compatibility guarantee. Hardware tests require explicit ignored tests and manual checklist.
Deliverable is a native executable, GitHub Actions artifact, SHA256. A source archive is only an interim checkpoint if compilation/publication are blocked.

