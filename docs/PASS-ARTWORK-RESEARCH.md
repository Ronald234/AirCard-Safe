# Nonpayment pass artwork: research component

This component selects visual resource candidates from caller-supplied classic pass metadata and a matching root file inventory. It neither obtains either input from an iPhone nor proves their provenance. It is not linked into AirCard-Safe.exe.

## Why the bank-card writer is insufficient

The reviewed hoicau/AirCard-Linux core at acaf415b935c042efb3691e928504150cb34e9e3 allows only cardBackgroundCombined@3x.png, cardBackgroundCombined@2x.png and cardBackgroundCombined.pdf for WalletArtwork. Its native device crate directly depends on its Linux adapter. Copying this implementation without a Windows adapter and different resource selection would not create a generic NFC-pass editor.

## Implemented

- Explicit classic layouts: generic, storeCard, coupon, boardingPass and eventTicket.
- Select existing root PNG variants only: logo plus thumbnail (generic), strip (store/coupon), footer (boarding), or the permitted event families.
- NFC is only an observed metadata object, not a verified credential or access classification. Its payload, public key and authentication fields are consumed without being retained in output.
- Fixed errors; no raw parsing errors, identifiers or arbitrary filename values in the result.
- Bounded metadata (256 KiB), top-level fields (256), inventory (512), filenames (128 bytes).
- Reject duplicate/conflicting layout metadata, unsafe root paths, case aliases and Windows trailing-dot/space aliases.
- Reject modern poster/style declarations instead of assuming their classic fallback is the active appearance.
- Reject event-ticket strip images combined with background or thumbnail at any scale.
- No file writes, native calls, new dependencies or device write capability.

Tests use synthetic data only. The input metadata remains owned by the caller; this does not promise secure erasure of caller memory. The component is a schema inspector, not a signature, full pass, NFC payload or PNG validator. Root-only selection deliberately excludes localized images, icons, modern poster resources and bank-card backgrounds.

## Remaining work before actual customization

1. Obtain real, correctly bound metadata and artwork inventories through a reviewed Windows acquisition method. The scanner aggregates cannot supply this.
2. Adapt the transaction engine and transport; preserve originals and every auxiliary preimage before replacement. The three-slot storage prototype is not yet a general pass backup.
3. Validate resource-specific rendering and replacement, conflict detection, interrupted transfers and reconnection recovery on a physical test device.
4. Preserve credentials and authentication without assuming that unchanged metadata makes a modified signed package valid. Re-importing a changed pass requires valid issuer signing. The cache-editing route is a separate, unverified mechanism.

No real NFC pass was edited, and no compatibility with a gym or merchant is claimed. This branch adds a reusable resource-selection component, not a downloadable NFC editor.

## Primary references

- https://github.com/hoicau/AirCard-Linux/blob/acaf415b935c042efb3691e928504150cb34e9e3/crates/core/src/customization.rs
- https://github.com/hoicau/AirCard-Linux/blob/acaf415b935c042efb3691e928504150cb34e9e3/crates/device/src/lib.rs
- https://developer.apple.com/library/archive/documentation/UserExperience/Conceptual/PassKit_PG/Creating.html
- https://developer.apple.com/documentation/walletpasses/creating-a-poster-generic-pass
- https://developer.apple.com/documentation/walletpasses/creating-an-event-pass-using-semantic-tags

No upstream implementation code copied. No user diagnostics, device version or private membership information published.
