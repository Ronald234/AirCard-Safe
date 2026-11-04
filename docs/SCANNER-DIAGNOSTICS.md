# Scanner and minimized diagnostics (revision 4)

Discovery is session-only. Seeing an identifier does not prove payment, NFC, barcode, access-control capability, or compatibility with any writer. An opaque ID or issuer name is never evidence of pass type. The pre-existing explicit Apple Card/AppleCard rendering context remains an incompatible static-artwork observation, not a guess based on an identifier. The legacy `PaymentCard` variant means a concrete card-cache path candidate, not a verified payment instrument.

Explicit `passType` / `passKind` fields can distinguish generic, loyalty, transit and access passes. Standard `generic`, `storeCard` and `boardingPass` JSON layout keys are also considered where present. A known nonpayment layout is read-only; opaque selections and Dashboard events stay unknown. Conflicting specific types become unsupported. Subsequent unknown/path events cannot erase stronger read-only evidence. No private credentials, tokens, keys or account data are parsed or exported.

## Primary-source review

Reviewed through the GitHub connector, pinned rather than assuming default-branch content:

- [dhava-gautama/AirCard-Windows scanner, commit 8df5b1664aff7e817b90ad11a15eb8a6ebbef0e4](https://github.com/dhava-gautama/AirCard-Windows/blob/8df5b1664aff7e817b90ad11a15eb8a6ebbef0e4/src/scanner.rs).
- [Lumid-Off/AirCard-Windows scanner, commit 88131e301c88fd9238cbeff9d091967c3ddbf10f](https://github.com/Lumid-Off/AirCard-Windows/blob/88131e301c88fd9238cbeff9d091967c3ddbf10f/src/scanner.rs).

The fork adds two `Dashboard loading` formats: `: for ID, ...` and `: ID - ...`. Revision 4 recognizes these only with live Wallet process/framework context, preserves the observed identifier including padding, and treats them as unknown candidates. We do not import the fork's global Express configuration inventory, bare base64 matching, identifier padding changes, persisted card metadata, raw log output, or compatibility claims. The upstream scanner confirms the existing path and explicit selection approach but is not evidence that all pass types emit identifiers.

## Limits and reports

The scanner bounds line, record, frame and list sizes and caps each event and session at 256 identifiers. Literal selected-pass events preserve short or dotted IDs as observed; they never become path-based writer targets. Malformed or truncated frames fail closed. Unified logging falls back to legacy logging only during startup. A scan has a deadline and cancellation control. Services are opened for log observation only; discovery does not read Wallet packages or write phone data.

Diagnostic JSON has a fixed schema of closed enums, booleans and saturating counters, with no freeform string field, timestamp, device ID, issuer, card ID/hash, path, raw error, pass description or raw log text. Only this typed schema is accepted by the export function. Raw syslog is processed transiently and is never persisted or exported. Diagnostics are finalised even on startup failure, stream failure, cancellation or deadline expiry.

Interpretation is deliberately limited:

- No Wallet event observed: the scanner cannot determine why; absence does not prove that the pass is absent.
- Wallet events without usable identifiers: the event was visible but does not yield a usable discovery ID.
- Redacted events: iOS marked some event data private; the scanner does not recover it.
- Unsupported or specifically classified nonpayment pass: visible evidence exists, but it remains read-only.
- Source unavailable or invalid frames: observation coverage was incomplete; this is not evidence about the pass itself.

Negative native receive results increment `receive_unavailable` and mark observations incomplete. Their cause is unknown; the report does not label them as a timeout or disconnection. Counts are aggregate observations, not a complete inventory or count of unique passes. Explicit kind-field fixtures are synthetic schema examples, not a claim that a particular iOS version emits these fields. Synthetic fixtures exercise detection, privacy and parser limits without recording a person's pass metadata. No physical-device compatibility claim follows from these tests.

