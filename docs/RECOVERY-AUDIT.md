# Recovery audit — AirCard Safe revision 4

## Decision

Applying and restoring Wallet artwork are blocked in the backend before opening a device session, reading a supplied backup, staging files in Media, editing Books, synchronizing assets or cleaning paths. Revision 4 removes the previous live private writer. There is no acknowledgement, manifest flag, command-line option or cache-refresh bypass. Local image preview and export do not constitute recovery or a Wallet backup.

AFC on a stock device exposes `/var/mobile/Media`; it does not provide authenticated reads of the actual originals in `/var/mobile/Library/Passes/Cards`. A generated image, a local file with a matching SHA-256, a successful AirTraffic submission and restoration of selected Books files establish neither original provenance nor recoverability. No trusted, non-destructive recovery capability has been implemented or tested. Consequently no Wallet mutation is permitted.

## Pinned upstream sources

Read through the connected GitHub integration. Repository: `dhava-gautama/AirCard-Windows`; reviewed commit: `8df5b1664aff7e817b90ad11a15eb8a6ebbef0e4`.

| File | Git blob SHA | Source |
| --- | --- | --- |
| wallet_backup.rs | 02976ac64b00dc2c2cbe9dbe3903b04b7d355b02 | [Pinned source](https://github.com/dhava-gautama/AirCard-Windows/blob/8df5b1664aff7e817b90ad11a15eb8a6ebbef0e4/src/wallet_backup.rs) |
| flasher.rs | d82fc9ae3bd6f159be771cf44bca35fb519a1d6d | [Pinned source](https://github.com/dhava-gautama/AirCard-Windows/blob/8df5b1664aff7e817b90ad11a15eb8a6ebbef0e4/src/flasher.rs) |
| airlift.rs | 4342d5466525fdfcf20acc173385b55fe67afd4b | [Pinned source](https://github.com/dhava-gautama/AirCard-Windows/blob/8df5b1664aff7e817b90ad11a15eb8a6ebbef0e4/src/airlift.rs) |
| LICENSE | 14fac913ccf80234b1848540089a3bbcb6e5283d | [MIT license](https://github.com/dhava-gautama/AirCard-Windows/blob/8df5b1664aff7e817b90ad11a15eb8a6ebbef0e4/LICENSE) |

Existing MIT notices are retained. This audit describes code paths, not results of device experiments.

## Data-loss findings

1. **Capturing a backup itself mutates protected artwork.** `read_protected_file` explicitly moves an original into an `airlift-recovered-` Media path, reads it through AFC, then uses the writer to put the bytes back. The original may be absent from its destination during that interval. This is not a safe read-only acquisition.
2. **The only durable recovery copy can be removed before write-back.** In `read_protected_file`, `Ok(Some(data))` calls `write_system_files_batch`. Its `write_system_files_batch_once` first calls `cleanup_airlift_staging`. That helper removes all entries with `RECOVERED_PREFIX`, including the just-exported original, before staging its replacement. The in-memory `data` can remain, but a disconnect, process exit or failed write at that point can lose the persistent recovery copy while the protected original is still absent. Local `save_original_asset` only runs after `read_protected_file` returns successfully.
3. **Cleanup removes Books state before snapshot.** The same helper deletes tracked Books files, then the writer snapshots them. Its snapshot can therefore record absence instead of the user's preceding Books state. Errors from cleanup are reduced to counts; several later cleanup/restoration errors are discarded.
4. **Recovery is not verified at the protected destination.** Presence checks for staging and a successful sync do not read back protected Wallet bytes. The recovered Media copy is removed after reported write success without destination verification.
5. **Partial local backups are accepted.** `backup_exists` uses `any`, `load_original_assets` accepts any nonempty subset, and capture continues after individual errors and accepts one saved asset. An incomplete folder can suppress subsequent capture. Neither provenance, complete asset coverage nor recovery is established.
6. **The local replacement helper is not lossless.** `atomic_write` writes a predictable temporary path, removes an existing destination, then renames. A rename failure after removal loses the destination. The earlier `is_file` check in `save_original_asset` does not exclude a concurrent creator or writer. This is a code-level race/data-loss possibility, not an observed incident.
7. **Cache refresh destroys or overwrites existing cache contents.** This upstream revision relocates protected cache leaves and deletes Media staging; earlier Safe revision 3 wrote `corrupted` to six fixed leaves. Neither is covered by the three-artwork local format. An artwork-only folder cannot establish a full rollback for these mutations. Upstream retry/partial-success paths also obscure failure.

## Local format and retained code

The legacy v1 local format binds pseudonymous device/card values and verifies three file hashes and signatures. These checks can detect ordinary accidental mismatch or tampering; anyone able to author the files can also author their hashes and binding. It is not an attestation of origin. A read-only inspector may inspect those files as unverified local data, but the restore loader refuses to authorize them. Existing backup files are not deleted or replaced.

Pure path allowlists, byte comparison and cleanup error aggregation remain for offline regression checks and audit history. They are not a route to device mutation or proof that recovery exists. Low-level transport/archive code retained elsewhere is not called by either backend entry point.

## Verification boundaries

Offline regressions invoke the real apply/restore functions from an integration crate with no device, AFC, AirTraffic or staging modules. They assert refusal and zero log callbacks, including valid-looking artwork and invalid arguments. Backup cases cover a locally fabricated complete manifest, wrong device/card binding, tampering, incomplete assets and foreign formats. Cleanup tests check that the first phase failure stops later submissions, both cleanup phases are attempted, and errors remain visible. No tests write to or restore a phone. Red-first recovery tests demonstrate that revision 3 accepted a self-authored complete local backup and did not return the required recovery refusal at its entry points.

A future enabled implementation must independently establish the actual original assets, complete coverage of everything it may mutate, durable preservation before any mutation and a validated recovery mechanism with protected-destination verification. An inferred or fabricated proof must never unlock the operation. This revision does not claim those capabilities.
