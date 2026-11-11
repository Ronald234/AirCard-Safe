# Native NFC artwork work — 2026-11-11

Status: unresolved. This branch is not a release with NFC artwork editing enabled.

## Changes

- Accept anchored `PassbookUIService` and `PassKitCore` event sources. Preserve the existing identifier validation and reject redacted identifiers. Three synthetic regression tests cover the newly accepted sources, an unrelated logger embedding a Wallet source name, and `<private>`.
- Construct AFC with a borrowed socket (`AFCConnectionCreate`, `closeOnInvalidate=0`). The AMD service retains socket ownership. AFC closes before the service is invalidated. An ignored physical regression reads Media directory metadata and repeats open/read/drop five times; it never prints directory contents or device identifiers.
- Run Windows format, locked tests, correctness analysis, locked release compilation and PE verification on this research branch. No executable artifact is published by this workflow because the requested editing capability remains unresolved.

## Scope of verification

Automated tests do not prove a pass is detected on a physical iPhone. The physical regression is ignored in CI. The AFC implementation is currently test-only archival code, not a release editing backend. The scanner change does affect the app. A release build is not proof of device mutation or recovery.

Local Linux recovery-store tests: 21 passed. These validate local storage and synthetic pass inventories, not acquisition of protected files from an iPhone.

Windows validation: https://github.com/Ronald234/AirCard-Safe/actions/runs/42942595397
Consult the actual run conclusion; a pending run is not a pass.

## What still prevents editing

1. Obtain the actual live identifier of the selected membership pass. Source recognition cannot reconstruct a value iOS replaces with `<private>`. An NFC activation/payment identifier is not automatically the filesystem identifier of a membership pass.
2. Acquire and identify the existing visual resources without changing credentials. Standard AFC access to Media does not imply access to protected Wallet files.
3. Implement and verify a durable device-specific recovery transaction before enabling artwork replacement. Recovery must survive interruption during acquisition, writing, verification and cleanup, and retain the surviving originals when recovery cannot complete.
4. Verify replacement and restoration of the selected physical pass, plus its continued normal use. None of these outcomes follows from passing the parser tests.

`src/flasher.rs` still rejects apply and restore before device access. This is an explicit missing backend, not a claim that toggling a UI option fixes compatibility.

## External approaches checked

- The Mak5er scanner documentation explicitly acknowledges that `<private>` identifiers cannot be recovered by its parser: https://github.com/Mak5er/AirCard/blob/main/docs/guides/TROUBLESHOOTING.en.md
- The transit-card Service Mode report is specific to a Shanghai transit card; it does not establish an Anytime Fitness workaround: https://github.com/Mak5er/AirCard/issues/25
- hoicau's Linux recovery implementation uses a durable journal and moves originals to read them. Its `leaves()` list handles three `cardBackgroundCombined` images, plus separate cache files; this is not an inventory of generic NFC pass artwork: https://github.com/hoicau/AirCard-Linux/blob/main/crates/cli/src/wallet.rs
- The AFC constructor ABI reference is https://github.com/DerekSelander/mobdevim/blob/main/mobdevim/misc/ExternalDeclarations.h . The callback and context are null; AMD retains socket ownership.

No credential extraction, signing-key extraction, pass cloning, NFC payload replacement, whole-Wallet database moves or guessed identifiers are implemented by this change.
