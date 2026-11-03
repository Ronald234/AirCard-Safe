# Validation and hardware limits
## Automated coverage
- Wallet-only target allowlist; path traversal, slash, malformed padding and invalid identifiers.
- Original artwork -> .cache -> .pkcache order, exact selected-card cache leaves and marker, and no retry/continuation after any failed phase.
- Separate @3x (1536x969) and @2x (1024x646) PNG preparation.
- Native unified-log fragmented frames, lengths/endianness, complete multiline records, cancellation and malformed input rejection.
- Explicit incompatible membership evidence overrides inferred payment classification; an untyped selected-pass ID never becomes writable by itself.
- USB route policy rejects missing USB and ambiguous network advertisements.
- Byte verification rejects changed or empty data.
- Recovery coordinator attempts restore and cleanup after a write error and retains all failures.
- Backup unit tests (requires Cargo dependencies): complete/incomplete, wrong device and changed file.
- Windows SHA-256 known vector and random tokens (requires Cargo dependencies).
- Original image crop/PDF and scanner parser tests retained; image validation tests added.
- Physical USB enumeration is explicitly ignored in CI, never silently reported as a passing hardware test.
- PE verifier has malformed/truncated header regression fixtures. Fixtures are not real program builds.
- CI additionally checks the actual binary's PE32+ AMD64 GUI header, runs --self-check and briefly launches its GUI.

## Required physical test before a device-support claim
Use a consenting owner's noncritical test device and a separate verified full-device backup.
Record Windows edition/build, Apple driver version, iPhone model, iOS version/build, cable and tested source commit.
Confirm USB works with Wi-Fi sync disabled; test rejection when a network route is advertised.
First test read-only enumeration/scanning. Confirm no raw card/device identifiers are persisted.
Confirm applying is disabled until the user explicitly accepts having no verified Wallet backup or guaranteed restoration. No protected Wallet read through ordinary AFC should occur.
Verify all staged Media bytes before dispatch, and record the visual result after reopening Wallet separately from the transfer return value. Test on an unsupported version: a returned acknowledgement must not be presented as a verified artwork change.
Confirm all three phases appear, then force-close and reopen Wallet on the phone. Cache invalidation overwrites six protected cache files; neither regeneration nor reversibility is guaranteed. Do not claim the omitted-refresh correction solves the reported device failure until the phone's visual result is observed.
Open a membership/NFC pass while scanning; verify whether its identifier is exposed. Confirm untyped or incompatible passes remain read-only, including after later log events with the same identifier. Privacy-redacted fields may prevent discovery.
Check Books tracked state and staging cleanup. For an existing compatible artwork backup, verify local digests/binding and record visual restoration separately; protected destination readback is unavailable.
On a controlled test setup test interrupted staging, disconnected USB, missing/corrupt backup and failing cleanup.
Do not interpret a protocol acknowledgement, synthetic fixture or successful compilation as an iPhone result.
No such physical tests were performed in this session.

