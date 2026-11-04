# Recovery and Wallet diagnostics implementation plan

**Goal:** ship an independently buildable Windows application that broadens evidence-based Wallet discovery and explains failures while refusing any device mutation without verifiable original artwork and recovery.

**Architecture:** retain Rust, native Apple USB services and the existing bounded log decoder. Discovery is separate from write authorization. Diagnostic exports contain only fixed enums and aggregate counters. Neither a hash-shaped ID nor a local file checksum proves payment type, backup provenance or recoverability.

**User constraints:** USB only; no credential/key extraction or changes; no keypad functions; no invented identifiers; no physical compatibility claims; unattended implementation and Git commits; Windows executable and SHA-256.

## Tasks

- [x] Compare pinned original, improved fork and current Safe code; record useful transport improvements and reject unsafe backup/deletion behavior.
- [x] Add failing tests for backend apply/restore without proven recovery; remove the unsafe write path and provide an explicit fail-closed reason.
- [x] Add failing fixtures for additional PassKit/NFC/pass selection events, explicit generic/loyalty/transit/access classification, hidden identifiers and secret-free diagnostics; implement bounded parsing and aggregate reports.
- [x] Integrate safe diagnostic UI/export, classification labels, local image preparation/export and permanent recovery status. Preserve public API compatibility where practical.
- [x] Validate transport changes with protocol-level tests; no unverified Grappa fallback or session behavior may authorize writes.
- [x] Add compatibility/security documentation and Windows CI checks/artifact packaging with SHA-256 and real PE validation.
- [x] Run formatting, cargo test --locked, cargo build --locked --release, security/static checks and actual Windows binary/self-check verification. Review changes and correct failures.
- [ ] Commit and publish source, run Windows Actions and report its actual conclusion, downloadable artifact, checksum and remaining physical-device/recovery limitations.

## Review focus

- Absent/redacted events must remain unknown and never become fabricated inventory.
- Explicit incompatible/ambiguous evidence must never increase write permission.
- Diagnostic JSON must not contain raw logs, paths, identifiers, tokens or user/device metadata.
- Backups of Media/Books are not backups of protected Wallet artwork; no blanket directory deletion or cache corruption is acceptable.
- A successful build or USB log connection is not proof of customization, NFC access or restoration on an iPhone.
