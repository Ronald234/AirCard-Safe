# Additional upstream code review: membership artwork

Reviewed Mak5er/AirCard at 52ba05dca4bdc590672c19d554701f4438e20338.

## Concrete code findings

- wallet_catalog.py: build_catalog reads the Mac's ~/Library/Passes/RemoteDevices.archive and Cards/*.pkpass/pass.json. Membership detection recognizes generic, storeCard, boardingPass, eventTicket and coupon layouts. This is a local macOS cache, not a USB enumeration API available to Windows. The catalog explicitly warns about incomplete/stale caches.
- Sources/WalletDiscovery.swift: distinguishes scan-confirmed IDs and cached memberships awaiting confirmation. It recognizes additional path forms, including a bare /ID.pkpass resource path. It also accepts global ID lists and fallback tokens; those are insufficient evidence of a current, correctly bound target and should not be imported wholesale.
- card_assets.py: build_card_assets always produces cardBackgroundCombined@3x.png, cardBackgroundCombined@2x.png and cardBackgroundCombined.pdf. Both PNG variants receive the same input bytes in this revision.
- aircard_backend.py: cmd_flash uses those fixed assets for its target. On batch failure it retries individual writes; it then removes display cache files. This function contains no acquisition and byte-verified rollback of the original Wallet artwork. A membership entry in the catalog is not proof that its asset layout supports this writer.
- docs/guides/TROUBLESHOOTING.en.md explicitly distinguishes a progress result from visible success and says the GUI has no automatic rollback/one-click restore flow.

Primary code:
https://github.com/Mak5er/AirCard/blob/52ba05dca4bdc590672c19d554701f4438e20338/wallet_catalog.py
https://github.com/Mak5er/AirCard/blob/52ba05dca4bdc590672c19d554701f4438e20338/Sources/WalletDiscovery.swift
https://github.com/Mak5er/AirCard/blob/52ba05dca4bdc590672c19d554701f4438e20338/card_assets.py
https://github.com/Mak5er/AirCard/blob/52ba05dca4bdc590672c19d554701f4438e20338/aircard_backend.py

## Relationship to previously reviewed alternatives

- tqmane/airliftbrowser-windows: its wallet.py explicitly requires paymentCard metadata, and its protected-region read lease can relocate the entire Cards directory. Not a verified AF membership solution.
- hoicau/AirCard-Linux and Max-Sum/AirWalletExport: journals and recovery state offer useful design references, but original acquisition still involves MOVE operations. Prior upstream hardware reports are not our Windows/AF validation.
- Pass creation libraries and QR membership-pass generators issue new signed passes. Their existence does not establish the ability to update another issuer's installed NFC pass while preserving reader acceptance.

## Engineering decisions

Useful follow-up: fixed structural diagnostics for unrecognized Wallet event shapes; separately record explicit pass-layout evidence and live-ID observations. Never infer payment or access technology from identifier shape. A future parser change must have synthetic regression tests and must not treat stale cached/global IDs as current inventory.

No writer is enabled by this review. The original-acquisition and verified-restoration gap remains. No public code reviewed here demonstrated a validated Windows customization-and-recovery path for Anytime Fitness artwork. This is a bounded source-review finding, not a claim that such a method is impossible or that no other code exists.

No upstream code copied. No user diagnostics or personal identifiers included.
