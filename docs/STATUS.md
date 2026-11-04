# Revision 4 validation status

Current source changes passed local Windows verification. GitHub Actions publication/verification is still pending; this file will be updated with the actual successful run and artifact after completion.

| Check | Actual result |
|---|---|
| cargo test --locked | 117 passed executions, 0 failed, 2 ignored physical USB tests |
| Main unit suite | 56 passed, 1 ignored |
| Engine integration suite | 1 passed |
| Recovery integration suite | 7 passed |
| Safety integration suite | 15 passed |
| Scanner integration suite | 38 passed, 1 ignored |
| cargo clippy --locked --all-targets -- -D clippy::correctness | Passed; nonblocking dead-code/style warnings remain |
| cargo build --locked --release | Passed after ZIP 9.3.0 update |
| PE verifier fixture suite | 8 passed |
| Actual release binary | PE32+ AMD64 Windows GUI |
| Release --self-check | Passed |
| Release GUI launch smoke | Passed, five seconds without an iPhone |

Some shared unit tests run again inside integration harnesses; 117 is an execution total, not a count of unique test definitions. Physical USB enumeration is repeated as ignored in two harnesses and is not counted as a pass.

Applying/restoring images is blocked before any device access because protected original artwork and recovery cannot be verified. The artifact will support USB diagnosis and local image design/export. The revision 3 artifact is historical and does not meet this recovery requirement.

No physical iPhone test, NFC compatibility, gym entry, payment, protected-image restoration or Windows 10 physical validation is claimed for revision 4. Apple Devices loading and native service behavior require physical/runtime verification. A complete fresh RustSec dependency audit remains pending; targeted ZIP advisories were reviewed and the inherited version upgraded. See SECURITY-REVIEW.md, COMPATIBILITY.md and RECOVERY-AUDIT.md.
