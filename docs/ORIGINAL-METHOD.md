# Original transfer method with bounded changes

The user requested retaining the original application method and removing risky behavior, instead of requiring a protected-file backup that ordinary AFC cannot obtain.

## Preserved from upstream

Upstream revision: `88131e301c88fd9238cbeff9d091967c3ddbf10f`, MIT license retained.

The writer uses the same multi-payload StreamingZip archive structure, protected Wallet target, three artwork filenames, Books synchronization manifest, and AirTraffic link-then-payload routing as upstream `write_system_files_batch`. The original artwork -> `.cache` -> `.pkcache` sequence is restored, including the six invalidation-marker cache writes. The original Apple device FFI and image preparation engine remain the basis. This is a constrained version of that method, not a new privileged read mechanism.

## Changes and removed behavior

- Password-keypad features and general-purpose system-file writers are absent.
- USB only; ambiguous network advertisements rejected and transport rechecked before dispatch.
- Exactly three Wallet image assets, validated target components, size limits and PNG/PDF signatures. No arbitrary protected destination or pass metadata write.
- All Media payload bytes and the staged Books manifest must match before dispatch.
- Checked random token generation; no raw card/device identifiers or syslog persisted automatically.
- No automatic fallback from a failed batch to individual writes. Such a retry can repeat a partial mutation.
- Cache invalidation is restored, limited to the selected identifier and exactly three leaves in each of two cache directories. It uses the original `corrupted` bytes to request cache regeneration. No arbitrary cache file or metadata write is exposed. Regeneration and its safety cannot be verified through AFC.
- No recursive staging deletion through symlinks. Cleanup failures remain visible alongside transfer and Books restoration errors.
- No attempt to read protected Wallet originals or destination bytes through Media AFC. No automatic Wallet-backup claim.
- Separate UI messages for detection and transfer. A completed transfer is explicitly unverified at the destination and visually.

## User-visible risk

The original method relies on private device services and iOS behavior. It is not an Apple-supported customization API. It can fail or do nothing on an unsupported iOS version. The earlier Safe build omitted the cache invalidation sequence; the user reported no visible Apple Card change on iOS 27.0. The present correction fixes that source-level omission, not a physically verified outcome.

The user must accept applying without a verified Wallet backup. Restoration of artwork is not guaranteed. The optional backup loader verifies existing local artwork digests and a pseudonymous card/device binding only; submitting those bytes does not prove restoration on the phone.

Operations are non-atomic. Disconnect, process termination or power failure can leave partial artwork changes or Books state. Only the six tracked Books files are captured in memory; this is not a full Books backup and does not survive process death. The private synchronization API can block. These limits were not eliminated by restoring the original method.

## Validation

Local policy tests exercise incomplete/duplicate/wrong-signature artwork, target escapes, changed or short staging bytes, transport read errors, USB policy and combined recovery failures. Windows CI must run the full locked test suite, locked release build, actual PE header/self-check and launch smoke tests before delivering an artifact.

No physical iPhone test was performed. No actual artwork apply, restoration, NFC gym credential or Windows 10/11 hardware compatibility result is claimed. The historical [access review](ACCESS-REVIEW.md) describes the rejected protected AFC backup design.
