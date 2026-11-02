# AirCard Safe Implementation Plan
**Goal:** Portable Windows Wallet-only executable with explicit verification limits.
**Architecture:** Retain upstream Apple FFI and image engine; replace UI, scanner persistence and write orchestration.
**Tech Stack:** Rust 2024, egui, Windows Apple device services.
**Spec:** ../specs/2026-11-02-aircard-safe-design.md
## Global constraints
MIT attribution; no keypad features; USB only; no development tools for end users; no invented hardware validation.
## Review focus
Unplug or network advertisement during writes; partial/corrupt backup; malformed path; hidden cleanup failure; excessive image/log input.
## Tasks
- [ ] Regression tests for target paths, USB selection, verification and image limits; attempt locked suite before implementation.
- [ ] Implement strict Wallet-only backend and in-memory UI, verify backup against phone before staging.
- [ ] CI windows-latest: cargo test --locked, cargo build --locked --release, PE x64 check, hash, smoke test, artifact.
- [ ] Verify locally, inspect diff, retain explicit blocked checks, publish only with valid GitHub access.
## Execution ledger
Isolated aircard-safe branch. User supplied detailed spec and autonomous execution preference; no design approval pause.
Direct git/network blocked. Source imported through GitHub connector at pinned upstream revision (not a full upstream history clone).
GitHub CLI token invalid; connector reports zero writable repositories; browser signed out.

Source implementation and workflow prepared. Focused standard-library tests:
9 passed; PE verifier: 8 passed; format/metadata/whitespace checks passed.
Full Cargo test/build and all CI/publication/delivery steps remain BLOCKED.
Independent source review: corrected source capture, restored empty Books bytes,
and retained unknown-presence errors. Tested generic recovery logic with fault injection.
Ruling: strict complete-artwork backup/read-back is required; this can refuse
stock iPhones where AFC cannot read Wallet artwork. No bypass was added.
Ruling: no detached AirTraffic timeout writer; private API blocking remains a
documented limitation. No claim of crash-safe rollback.

