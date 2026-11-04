# Anytime Fitness artwork: concrete research route

## Vendor evidence

Dot Origin's Anytime Fitness case study identifies the deployment as VTAP100 NFC Wallet membership passes. It describes Apple VAS / Google Smart Tap, and states that Anytime Fitness generates its own passes and developed its Apple/Google issuance and update integrations.

Primary source:
https://www.vtapnfc.com/downloads/VTAP-NFC-readers-provide-world-wide-gym-access-control-Anytime-Fitness.pdf
Relevant sections: "The solution: A customised VTAP100 mobile wallet NFC reader" and the final page on issuer integrations.

This establishes the documented deployment, not the exact implementation or current contents of every individual installed pass. Do not automatically treat a gym pass as a Secure Element access key, or infer its technology from a brand alone.

## Viable routes and prerequisites

1. Issuer update: AF's authorized pass-generation/update pipeline can supply a newly signed pass with changed artwork while preserving the membership identity and NFC settings. This is the documented platform route. No evidence was found of a public member-facing arbitrary-image editor or a public third-party API that authorizes AirCard to do it.
2. Local artwork/cache research: determine the actual visual asset layout and whether narrowly scoped originals can be acquired and restored. The documented payment-card combined-background method does not establish support for AF's membership layout. This route remains unverified and cannot enable production writes.
3. A locally created visual replica is not proof of a working AF access pass. Re-signing or rebuilding another issuer's pass is not a demonstrated way to preserve acceptance by the gym reader.

Apple signing reference:
https://developer.apple.com/library/archive/documentation/UserExperience/Conceptual/PassKit_PG/Creating.html
VAS membership reference:
https://developer.apple.com/wallet/loyalty-passes/

## Detection defect to address

In current scanner.rs, a Cards path whose identifier passes is_valid_card_hash can produce PaymentCard even without explicit payment-layout evidence. The diagnostic label payment_candidate must not be interpreted as a confirmed bank card. Hash shape alone cannot classify the issuer or pass technology.

The current diagnostic export aggregates classifications and does not retain an event-to-item trace. One unknown classification and one payment candidate need not represent two distinct passes. no_identifier means the parser extracted no item, not proof that iOS redacted the identifier.

A useful next diagnostic revision should distinguish fixed, allowlisted structural reasons (known event with redacted ID, invalid/missing ID, unrecognized event shape, explicit type evidence) without exporting raw logs, arbitrary strings, IDs, membership payloads or authentication fields. Session-only associations could distinguish reclassification from multiple items; they must not become stable identifiers.

## Status

Research findings only. No NFC payload, credential, issuer signature or device file was changed. No working AF artwork customization or recovery claim is made. Physical layout/acquisition/recovery evidence is still missing.
