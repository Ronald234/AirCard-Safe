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

Its tests can establish local integrity and failure behavior only. The component deliberately has no cleanup of originals. Windows ACL hardening and actual power-loss behavior remain research tasks. See its README for precise boundaries.

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

A subsequent Windows private-ACL experiment is incomplete and is not part of the tested commit or shipping application. The local Windows environment became unavailable during follow-up validation. Owner validation, defensive SID bounds and supported-token Windows tests remain pending; AppContainer refusal alone does not validate normal-user behavior. The published component's documented Windows privacy and durability limitations still apply.

No new executable enabling edits is delivered by this milestone. The production application remains unchanged and read-only. Actual original acquisition, auxiliary preimages, destination restoration and physical interruption/reconnect/rendering tests remain required before any writer can be enabled.
