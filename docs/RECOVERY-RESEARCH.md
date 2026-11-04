# Recovery investigation — progress and remaining block

The objective is to recover image customization without accepting unverified recovery. Applying and restoring in the Windows application remain blocked. This work does not change that authorization boundary.

## New sources examined

| Source | Immutable revision | Finding |
|---|---|---|
| hoicau/AirCard-Linux | acaf415b935c042efb3691e928504150cb34e9e3 | Durable operation journals, exact readback, local locking and recovery states improve interruption handling; artwork is still temporarily moved to Media. |
| Max-Sum/AirWalletExport | b9aef5c0a495bfffffd286d288aaebfc0dfa5cbb | ADR 0002 explicitly describes move/read/restore and saving recovery state before moving. This is not non-destructive acquisition. |
| AntisKong/AirCard-Windows | 6e5962c4a5f20b462d46f6dafa031eba5b8c40db | airlift_read.rs retains recovered copies on certain failures, but still moves originals and deletes copies after reported write-back rather than independently verified destination restoration. |

Primary code references:

- https://github.com/hoicau/AirCard-Linux/blob/acaf415b935c042efb3691e928504150cb34e9e3/docs/WALLET-TRANSACTIONS.md
- https://github.com/hoicau/AirCard-Linux/blob/acaf415b935c042efb3691e928504150cb34e9e3/crates/cli/src/wallet.rs
- https://github.com/Max-Sum/AirWalletExport/blob/b9aef5c0a495bfffffd286d288aaebfc0dfa5cbb/docs/adr/0002-export-is-move-read-restore.md
- https://github.com/AntisKong/AirCard-Windows/blob/6e5962c4a5f20b462d46f6dafa031eba5b8c40db/src/airlift_read.rs

These are source findings. Upstream hardware reports are not our Windows validation. This investigation did not connect to or modify an iPhone.

## Official backup alternative

Apple states that computer backups exclude Apple Pay information/settings: https://support.apple.com/108771 . That statement does not enumerate every artwork file, so it also does not establish whether any specific image exists in a particular backup. An ordinary iTunes/Apple Devices backup cannot be assumed to be a verified backup or restoration mechanism for these protected images. No complete phone backup is requested or uploaded, and encrypted-backup passwords or keys are not extracted.

## Implemented first component

`research/recovery-store` is an isolated local-byte storage experiment, not linked to the shipping executable. It preserves supplied bytes before sealing a local integrity manifest, never overwrites an existing capture, retains partial captures on injected failures and refuses changed/incomplete/foreign snapshots. It carries no device-write permission, authenticates no provenance and has no native device transport dependency. No upstream implementation code was copied; the design is independently implemented and references the findings above.

Its tests can establish local integrity and failure behavior only. The component deliberately has no cleanup of originals. Windows ACL provisioning is addressed in the follow-up below; adversarial parent-path replacement and actual power-loss behavior remain research tasks. See its README for precise boundaries.

## Conditions before enabling a writer

1. Obtain actual original image bytes through a reviewed method. A non-destructive read is the preferred research route. A MOVE-based candidate would instead need a durable recovery intent saved before acquisition and demonstrated interruption/reconnect recovery; the reviewed source alone is not that evidence.
2. Preserve originals and every auxiliary preimage a chosen method would change, including cache/Books state if applicable. The three-slot laboratory alone is insufficient for that scope.
3. Validate restart/reconnect recovery, exact destination readback, concurrent-change conflicts, cleanup ordering and loss of power on Windows with a physical test device.
4. Distinguish original acquisition, local integrity, destination restoration and visible Wallet rendering. Evidence for one must never authorize the others.

Until those conditions are met, re-enabling the old unbacked writer would bypass the user's recovery requirement. No functional editing fix or NFC compatibility is claimed.

## Verified research milestone

The research source at `a96be1087b66c0605bf9770a223a43f3a9ebde1f` passed [GitHub Actions run 41769323051](https://github.com/Ronald234/AirCard-Safe/actions/runs/41769323051):

| Check | Result |
|---|---|
| Windows local-storage tests | 8 passed; no ignored tests |
| Linux local-storage tests | 10 passed; no ignored tests |
| Storage formatting, Clippy with warnings denied, locked release build | Passed on both platforms |
| Unchanged Windows application regression | 117 passed test executions; 2 physical-device tests ignored |

These counts include repeated shared-module executions in the application suite. Independent code review identified a Unix FIFO blocking issue; bounded, nonblocking ordinary-file reads and a regression test addressed it before this successful run. An earlier Windows-only lint failure was corrected before the successful run.

The tested component checks supplied local bytes, corruption, incomplete captures, unexpected files, binding mismatch and preservation after injected interruption. It does not obtain phone originals, restore protected destinations or demonstrate visible image changes.

The Windows environment later became available. The follow-up experiment now provisions private user/SYSTEM DACLs, checks directory/file owners and bounds ACE SID data before native conversion. Targeted rejection tests, strict Clippy and a locked release library build passed locally. The full local suite remains red in AppContainer: the interruption test and six capture-dependent integration tests fail because capture is refused before directory creation. No test is skipped to hide that limitation. Supported-context Windows CI has now passed all 12 storage tests, formatting, Clippy with warnings denied and the locked release library build. Linux passed all 10 storage tests and the same checks. This remains isolated research, not part of the shipping application.

No new executable enabling edits is delivered by this milestone. The production application remains unchanged and read-only. Actual original acquisition, auxiliary preimages, destination restoration and physical interruption/reconnect/rendering tests remain required before any writer can be enabled.

## Windows permission follow-up

Source: `d4ab576cc8f82116e75c2a0f89b5a293c4adae73`. All three jobs completed successfully: Windows storage (12 tests), Linux storage (10 tests), and unchanged application regression (117 passed executions, 2 physical tests ignored). [CI run 41930922859](https://github.com/Ronald234/AirCard-Safe/actions/runs/41930922859) exercises normal user-token directory creation and inherited file permissions; its Windows test asserts that CI is not taking only the AppContainer-rejection path. Owner policy rejects other standard accounts and trusts the current user, SYSTEM and Administrators. Same-account processes and administrators remain outside this discretionary-access-control boundary; data is not encrypted.

The owner and SID-extent regressions were observed failing before the checks were implemented, then passing. Independent follow-up review found no remaining critical or important issue for research publication, subject to supported-context CI.

Local AppContainer full-suite failures are retained explicitly below. Each capture-dependent test is refused before directory creation; these are not reported as local passes or silently ignored:

- `failure_after_first_flushed_asset_retains_bytes_and_does_not_seal`
- `restart_preserves_exact_bytes_and_explicit_absence`
- `an_existing_directory_and_its_files_are_never_overwritten`
- `incomplete_or_corrupt_capture_is_rejected_without_deleting_the_surviving_copy`
- `wrong_binding_and_changed_original_are_rejected`
- `unknown_manifest_fields_cannot_assert_verified_recovery`
- `unexpected_files_in_an_absent_slot_are_a_conflict`

All seven above pass in the supported Windows CI context. No physical phone test, original acquisition or restoration is performed by this suite. The application's writer stays blocked.
