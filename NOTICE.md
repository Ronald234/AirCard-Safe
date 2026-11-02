# Origin and license

Derived from Lumid-Off/AirCard-Windows under its MIT license.
Pinned upstream commit: 88131e301c88fd9238cbeff9d091967c3ddbf10f.
Original LICENSE retained without changes.
Credits: Lumid-Off, mak5er, 0xjohnnydev/airlift.

The original was read through the connected GitHub file API because a direct
git clone was blocked. Local Git history starts at an explicitly labeled
source import; it is not the upstream commit graph. Work is isolated on
the aircard-safe branch.

AirCard Safe modifications: Wallet-only UI, no keypad support, restrictive USB
route policy, in-memory card scanning, verified user-selected artwork backups,
strict write allowlist, read-back checks, explicit cleanup error handling, and
Windows CI/artifact packaging. This development checkpoint is uncompiled as
a complete application. No iPhone compatibility or complete-device backup is
guaranteed.
