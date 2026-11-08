# Pass artwork research verification

Source: 1ef9db49bb13c167018d843fffa90b00cbdfaa13.
Workflow: https://github.com/Ronald234/AirCard-Safe/actions/runs/42503922467

- Windows: 23 passed test executions across the complete research crate (5 + 5 + 6 + 7).
- Linux: 21 passed test executions across the complete research crate (1 + 5 + 6 + 9).
- Six new inventory tests passed on each OS, covering five classic layout families, synthetic NFC privacy, invalid/duplicate metadata, unsupported modern presentations, Windows aliases, event-image conflicts and bounded inputs.
- cargo test --locked, rustfmt, Clippy --all-targets -- -D warnings, and cargo build --locked --release all passed for research/recovery-store.
- Independent code review found three important issues; each was reproduced by a failing test and fixed. Re-review found no remaining critical or important issue.

Failure evidence retained in Actions:
- 42503362446: all four initial inventory tests failed against the placeholder implementation.
- 42503505572: functional tests passed; formatting failed.
- 42503676259: three regressions failed (poster fallback, Windows alias, event-image conflict).
- 42503811811: all functional tests passed; two formatting differences remained.
- 42503922467: both OS jobs passed every check.

Scope: a supplied-data inventory library, not a phone acquisition engine, image renderer, signed-pass editor, transaction engine or NFC writer. Neither the application nor its device-write gate changed. No new application executable was published. No physical device was connected or modified. The local execution host disconnected; validation used GitHub Actions.

The real-user objective remains unresolved: obtain verified same-pass original resources on Windows, render replacements, integrate safe recovery, and demonstrate visible customization while retaining NFC reader acceptance. Passing these synthetic tests is not evidence that a real NFC pass can yet be edited.
