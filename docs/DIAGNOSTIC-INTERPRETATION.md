# Diagnostic interpretation correction

The scanner retries negative native receive results; the private API does not provide a verified errno contract. A nonzero receive_unavailable count alone establishes neither a disconnect nor lost events. The UI now retains this uncertainty instead of asserting an incomplete capture. Actual stream failures and malformed frames still produce an incomplete-reading warning.

Known event markers without extracted items now take precedence over a general privacy warning. A marker does not establish that an identifier was present, that a particular pass caused the event, or that a parser extension is safe. No raw device text or private diagnostic captures are included in this repository.

Synthetic regression tests exercise mixed parser/redaction/retry evidence and terminal failure precedence. Local diagnostic bridge: five tests passed after two regression tests first failed against the previous explanation logic. Full application validation is performed by the Windows workflow for this commit.

## Editing remains unavailable

This corrects diagnostic explanations, not pass detection or device editing. The application still lacks verified original acquisition and destination restoration. Its apply and restore functions intentionally fail before opening a device session. More successful discovery events cannot satisfy this independent recovery requirement.

The reviewed MOVE-based exporters may offer research directions, but relocating originals is already a device mutation. They need durable recovery intent, preserved auxiliary preimages, restart/reconnect recovery and exact destination verification before integration. No physical experiment or phone mutation was performed for this change. Repeating aggregate-only diagnostic captures cannot establish these recovery properties or reveal the missing event syntax.
