# Correction of the omitted Wallet refresh phases

## Problem and implementation

The previous Safe executable submitted artwork and skipped the two cache invalidation batches that upstream calls immediately afterward. The user reported no visible Apple Card change on iOS 27.0. That establishes a failed visual test; it does not prove the omission was the only cause.

Further source review found a decisive limitation: the original Mac README explicitly states Apple Card uses dynamic vector rendering rather than static cached card artwork. The omitted-refresh correction therefore must not be advertised as making Apple Card work. The new UI excludes Apple Card and NFC membership passes, and explicit Apple Card log context is read-only. The source warning is at pinned commit `52ba05dca4bdc590672c19d554701f4438e20338`, README.md.

The new code restores the original order: three artwork assets, three `.cache` leaves, then three `.pkcache` leaves. Each phase uses the same upstream StreamingZip/AirTraffic batch mechanism. The cache target helper accepts only a validated selected card identifier, one of the two fixed directory suffixes, the three exact leaf names (`FrontFace`, `PlaceHolder`, `Preview`) and the exact upstream invalidation marker (`corrupted`). Arbitrary paths, metadata, credentials and keypad files remain outside the allowlist.

Each batch verifies Media staging bytes, restores the tracked Books files and reports cleanup failures. The next phase runs only after the preceding one returns without errors. No silent fallback or automatic individual-write retry remains. UI progress distinguishes the three phases and the acknowledgement explains the additional cache mutations.

## What remains unverified

Invalidation overwrites protected cache data with intentionally invalid bytes. It is not a non-mutating refresh API, verified deletion or guaranteed cache regeneration. Ordinary AFC cannot verify the protected destination or capture a complete Wallet backup. Power loss, cable removal, process termination and unsupported iOS behavior can still leave partial changes. No crash-safe rollback is promised.

Closing Wallet in the iPhone app switcher and reopening it is a required manual observation step, not proof from the transfer protocol. The final message requests that observation and does not claim a visual success.

## Tests

Regression coverage checks the exact cache targets/payloads, traversal and arbitrary-file rejection, all three phases in order, and stopping without retry when artwork, cache transfer, Books restoration or cleanup fails. Full locked tests, locked release build, PE/self-check and launch verification are required for delivery.

No new physical-device result is claimed by this source correction. Membership/NFC discovery and artwork compatibility are separate concerns: discovering an identifier must never authorize payment-card artwork writes to an unverified membership pass.

The image preparer also now generates distinct @3x (1536x969) and @2x (1024x646) PNGs rather than sending the @3x bytes under both filenames. The resolution regression checks both decoded images. This is an asset-format correction, not evidence of a visual result on the phone.
