# Wallet access review

## Outcome

The released Windows executable builds and starts, but its protected Wallet backup/readback design is not functional on stock iOS. The current failure must not be described as a user image problem, a successful backup, or a verified iPhone customization result. No executable fix or physical-device validation is claimed by this documentation update.

## Evidence

- `src/afc.rs` opens `com.apple.afc`, not a privileged root-filesystem service.
- Ordinary AFC exposes `/var/mobile/Media` as its root. It does not expose `/var/mobile/Library/Passes/Cards`. See the [libimobiledevice ifuse documentation](https://github.com/libimobiledevice/ifuse/blob/master/README.md#usage).
- `src/wallet_backup.rs::capture_original_card` searches the latter protected path. It cannot establish a verified backup using this transport.
- `src/flasher.rs::write_artwork` also attempts destination verification using the same inaccessible path. Merely removing the backup condition would leave this second architectural error and allow unverified modifications.
- `src/flasher.rs::apply` calls capture before staging or writing artwork. The reported capture error occurs before that operation writes artwork. This does not establish the state left by other software or earlier operations.
- `AfcClient::exists` collapses communication/access errors into false. The existing message cannot by itself distinguish a disconnected session from an inaccessible or missing path. This diagnostic weakness is still unresolved.

## Alternative examined, not incorporated

Reviewed the proposed protected capture mechanism in [upstream PR 60](https://github.com/Lumid-Off/AirCard-Windows/pull/60), pinned to `OlivierVencken/AirCard-Windows` commit `b5cb55fafeb180b354d06a87a494e842872915a6`, especially `src/effects/device.rs::capture_and_return_inner` and `capture_pairs`.

That mechanism exports protected originals to Media through privileged asset synchronization, saves the recovered bytes and attempts to restore them. Its own recovery code acknowledges that originals have been moved. Thus the initial capture already mutates protected device state, before a complete verified host backup exists. A journal and hashes help recovery but cannot guarantee restoration after a cable disconnect, process death or unsupported iOS behavior. Contributor-reported hardware results are not physical tests performed for AirCard Safe.

No code from that proposal was incorporated. No protected-file export, cache invalidation or device write was executed during this review.

## Requirements before claiming a working safer route

1. Establish a supported, repeatable way to capture originals and verify the destination on the exact iOS version. Distinguish a non-mutating read from a capture that moves files.
2. Persist all affected recovery state before mutation, including Books state; define recovery for partial export, interrupted synchronization and program restart.
3. Test missing files, access errors, partial operations, corrupt backups and recovery with a simulated transport, then physically validate on a disposable test device with nonessential passes.
4. Report remaining non-atomic operations and any unrecoverable failure mode explicitly. A passing CI run or successful synchronization return value is insufficient.

These requirements are unresolved. The existing write block remains enabled.

## Non-bank and gym credentials

Apple Wallet supports loyalty and membership passes, including gym passes. NFC membership passes require a compatible issuer and reader integration; see [Apple's membership and contactless documentation](https://developer.apple.com/wallet/loyalty-passes/).

AirCard Safe currently allows only three bank-style artwork filenames. It does not implement general membership-pass artwork layouts, provision NFC credentials, clone physical tags, or grant gym access. Changing artwork does not change the underlying credential. A gym-issued Wallet pass and a physical NFC card are different cases; neither has been physically validated here.
