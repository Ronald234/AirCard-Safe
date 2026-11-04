# Wallet NFC and original-acquisition investigation

## Scope and outcome

Artwork customization must preserve NFC credentials, authentication and signed issuer data. NFC identifies a contactless transport, not a common file format. No new acquisition or write method has been enabled. No iPhone was accessed in this investigation.

## Primary Apple sources

- https://developer.apple.com/wallet/loyalty-passes/ describes NFC loyalty/membership transactions using VAS and issuer-controlled artwork updates.
- https://register-docs.apple.com/apple-pay/access/program-guide/overview/access-passes distinguishes visual pass data from access credentials protected in the Secure Element.
- https://developer.apple.com/documentation/walletpasses/pass/nfc-data.dictionary documents NFC payloads and Apple's required entitlement.
- https://developer.apple.com/library/archive/documentation/UserExperience/Conceptual/PassKit_PG/Creating.html documents the package manifest and signature covering pass contents.

For a distributed signed pass, replacing an image changes the signed package contents. A legitimate updated package requires the authorized issuer's signing process. Replacing a derived on-device image cache is a different, unverified operation, not a general solution for editing signed NFC passes.

Neither an NFC transaction nor an opaque Wallet identifier proves the precise pass type, issuer, file layout, or replaceable asset. Do not classify a pass solely from a brand, its appearance or its use at a contactless reader. Access credentials and VAS payloads must not be extracted or changed to customize artwork.

## Additional implementation inspected

Repository: tqmane/airliftbrowser-windows, revision d54c8c268471cfb22a838af1adfff4bb7748fbab.

- https://github.com/tqmane/airliftbrowser-windows/blob/d54c8c268471cfb22a838af1adfff4bb7748fbab/src/airliftbrowser/backend/mobiledevice/wallet.py
- https://github.com/tqmane/airliftbrowser-windows/blob/d54c8c268471cfb22a838af1adfff4bb7748fbab/src/airliftbrowser/backend/mobiledevice/protected_container.py

Concrete source findings:
1. Wallet enumeration targets /var/mobile/Library/Passes/Cards. Its _card_summary explicitly rejects metadata without paymentCard. It therefore is not evidence of generic membership/access-pass support.
2. The artwork allowlist includes static fallback and parallax layers in addition to combined card backgrounds. This suggests additional asset layouts worth investigating, but does not prove compatibility or justify writing those assets blindly.
3. A read_only lease still stages Books state and invokes AirTraffic on the protected region. ProtectedRegionLease.open saves targetMoveStarted before the operation and subsequently distinguishes a remaining original from a moved original. The read_only flag is not evidence of non-destructive acquisition.
4. Its original-image read maps permission/unknown AFC failures to None. AirCard Safe must preserve the distinction between confirmed absence and unreadable content.
5. Moving a whole Cards region broadens the affected scope beyond one image or pass. Do not import that approach as a harmless discovery operation.
6. Journals and preimages are useful recovery concepts, but the source alone does not validate Windows interruption recovery or protected-destination byte restoration on our application.

No upstream code was copied.

## Remaining acceptance evidence

Before any device writer can be enabled:
- Establish the actual target pass type and artwork layout from explicit evidence; retain unknown/redacted states.
- Review a narrowly scoped original-acquisition method and save durable recovery intent before any operation that can move originals.
- Preserve every auxiliary preimage the method changes, including Books and cache state.
- Verify original bytes, interrupted/reconnected recovery and final destination restoration with a physical test device.
- Verify visible appearance and the pass's normal operation without inspecting or changing credentials.

The reviewed sources do not establish a universally safe method. Local storage tests, service acknowledgments and upstream compatibility claims do not satisfy these physical acceptance criteria.
