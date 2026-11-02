# Validation and hardware limits
## Automated coverage
- Wallet-only target allowlist; path traversal, slash, malformed padding and invalid identifiers.
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
Confirm all three Wallet artwork assets can actually be read over AFC. If not, expected behavior is refusal, with no staging or writes.
With readable originals, confirm backup digest verification, apply read-back and visual result after reopening Wallet.
Check Books tracked state, staging cleanup, restore read-back and visual restore.
On a controlled test setup test interrupted staging, disconnected USB, missing/corrupt backup and failing cleanup.
Do not interpret a protocol acknowledgement, synthetic fixture or successful compilation as an iPhone result.
No such physical tests were performed in this session.

