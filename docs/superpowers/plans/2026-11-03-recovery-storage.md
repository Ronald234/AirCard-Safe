# Recovery storage research plan

Goal: investigate a non-destructive original-artwork acquisition path and establish local storage behavior without weakening the revision 4 gate.

- [x] Inspect current source and reviewed MOVE exporters at immutable references.
- [x] Reject treating a general Apple backup as proof of protected artwork coverage.
- [x] Write failing local-storage and injected-interruption regressions.
- [x] Implement an isolated, immutable three-slot local snapshot component with bounded reads, no cleanup and manifest-last integrity checks.
- [ ] Verify local full tests, static checks and Windows CI.
- [ ] Obtain independent code review and publish a research branch with actual results.
- [ ] Resolve non-destructive original acquisition and Windows privacy/durability gaps.
- [ ] Physically validate capture, interruption, reconnect, destination restoration and rendering before changing the production writer gate.

The final two items are unresolved technical/hardware dependencies, not permission flags. Local hashes and journal presence cannot satisfy them.
