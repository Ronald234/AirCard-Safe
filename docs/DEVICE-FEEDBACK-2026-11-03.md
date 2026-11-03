# Static artwork compatibility limitations

## Apple Card

The original Mac README at commit 52ba05dca4bdc590672c19d554701f4438e20338 states that Apple Card uses dynamic vector rendering instead of a static cached face. The refresh/discovery corrections do not establish Apple Card support. See REFRESH-FIX.md and the pinned upstream README:
https://github.com/Mak5er/AirCard/blob/52ba05dca4bdc590672c19d554701f4438e20338/README.md

## Transfer versus visible result

A Book asset manifest and dispatched completion messages do not verify protected destination bytes or a visible Wallet refresh. The program reports an unverified transfer, never a proven visual result. The restored cache invalidation sequence fixes a source-level omission but is not guaranteed regeneration or restoration.

## Membership and NFC passes

The native scanner can observe structured pass identifiers. It does not establish that a pass supports the payment-card artwork filenames. Untyped and incompatible observations remain read-only, including after later weaker observations for the same identifier. A discovered credential is not a cloned NFC key or proof of gym-access compatibility.

## Public verification scope

Current build/test results are documented in STATUS.md. Private manual-test observations and personal device metadata are not published here. The automated physical-device test remains explicitly ignored; no universal hardware-support claim is made.
