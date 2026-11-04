# Transport and Apple runtime audit — revision 4

Audit date: 2026-11-03 (client date). Scope: source review and offline tests. No iPhone was queried, paired, synchronized, written to or restored during this audit. No device identifiers, device logs, personal device metadata or credential material appear here.

## Pinned sources

| Source | Commit | Evidence |
| --- | --- | --- |
| Dhava upstream | `8df5b1664aff7e817b90ad11a15eb8a6ebbef0e4` | GitHub `commits/main` resolved to this SHA during the audit; commit date 2026-10-09. [Source](https://github.com/dhava-gautama/AirCard-Windows/tree/8df5b1664aff7e817b90ad11a15eb8a6ebbef0e4). |
| Lumid original | `88131e301c88fd9238cbeff9d091967c3ddbf10f` | Files fetched by immutable SHA. [Source](https://github.com/Lumid-Off/AirCard-Windows/tree/88131e301c88fd9238cbeff9d091967c3ddbf10f). |
| Safe baseline | `835883e` | Local revision 3 checkout before this work; shared source baseline for the audit. |

The four files were read from both upstream pins. Existing MIT notices remain in the repository; registry-discovery bindings adapted from the pinned Dhava source retain that licensing context.

## Differences and disposition

| Area | Lumid original | Dhava latest | Safe revision 4 decision |
| --- | --- | --- | --- |
| `airtraffic.rs` negotiation | One-argument connection creation; `SendSyncRequest`; HostInfo after SyncAllowed. | `CreateWithLibrary`; HostInfo before SyncAllowed; explicit `RequestingSync` with a static replay Grappa and Book anchor; current-session number; PowerAssertion. | Reviewed only. Replay handshake is not activated or represented as validated. Protocol implementation is compiled for tests only. |
| Writer lifetime | Detached sync thread with caller timeout; Wi-Fi retry scaling. | Detached sync thread with 120-second caller timeout; 90-second ReadyForSync loop. | Existing joined execution and USB guards retained in archival source. Release has no AirTraffic implementation or ATH library dependency. |
| Batch dispatch | Multi-item archive support already exists; AssetCompleted pacing 400 ms then 60 ms. | Still supports multiple payload entries; uses 900 ms between AssetCompleted messages and additional pacing after streaming upload. | No batch write integration. Safety cannot be established by pacing or by a successful handshake. |
| `apple.rs` discovery | Conventional Mobile Device Support directories. | Also enumerates per-user AppModel registration for Apple Devices/Store iTunes, copies DLLs into a user-writable cache and reuses it on file existence. | Adds discovery of registered x64 Apple Devices/Store iTunes roots in the protected standard WindowsApps directory. Canonical paths are checked before selecting the package root or AMDS64 child. No copied DLL cache. |
| DLL loading | Global SetDllDirectory and default DLL loading. | Same global search mutation, including the staged writable cache. | LoadLibraryEx through libloading searches the installed DLL directory and System32 only. No working-directory, PATH or app-owned cache search; no process-global SetDllDirectory mutation. |
| `device.rs` routing | USB, Wi-Fi and automatic routes. | USB chosen by default but selected-device matching is not a USB-only proof. | Existing USB-only acceptance and rejection of ambiguous USB/network routes retained. Unknown transport remains Other. Existing response-size limit retained. |

## Why Grappa and batching remain unavailable

The Dhava source identifies its 84-byte blob as a replay from another project, not a newly negotiated device-specific value. This audit establishes what bytes its source sends; it does not establish that Apple accepts them on any particular iOS release. Neither upstream code nor unit tests demonstrate original-artwork recovery.

`ATHostConnectionReadMessage` is an opaque synchronous native call. The upstream loop's deadline is checked only between calls; it cannot cancel a blocked call. Returning on the separate caller's timeout can leave the writer running while cleanup proceeds. Extending that timeout does not fix the ownership problem. No undocumented cancellation ABI or speculative signature was introduced.

The archive already has multi-payload encoding, but send pacing does not prove destination integrity, completion, rollback or that every original remains recoverable. Revision 4 therefore excludes the private writer modules from the shipping application. Apply and restore remain blocked by the recovery gate described in [RECOVERY-AUDIT.md](RECOVERY-AUDIT.md).

## Apple Devices limits

Release requires CoreFoundation.dll and MobileDevice.dll, not AirTrafficHost.dll. ATH handles, symbols and loads are under `cfg(test)` solely for archival protocol tests. Pure tests do not load those libraries or contact hardware.

Store discovery accepts the Apple package family/publisher suffix and x64 layout in `C:\Program Files\WindowsApps`; alternative installation volumes and architectures are deliberately not guessed. Registry paths outside that protected location, publisher lookalikes and traversal paths are rejected. Registry reads bound allocation, enumeration count and UTF-16 decoding. Canonicalization rejects a package or AMDS64 junction that resolves outside the accepted package layout.

Some MSIX installations may deny direct DLL loading. The application reports the failure with standalone Apple runtime guidance rather than copying executables into a writable cache. Actual Apple Devices loading and scanner compatibility require a separately authorized hardware/runtime check; neither is claimed here. The package path filter is defense in depth, not an Authenticode or Windows package-signature verifier.

## Offline validation

The protected-package-root regression test was observed failing with a rejecting stub before implementation. It tests accepted synthetic Apple Devices/iTunes package roots and rejected writable locations, package-name lookalikes, publisher mismatches and traversal paths. No test input represents a real user device.

The protected-package-root test passed in both the main and scanner harnesses after implementation. Final integration outcomes are recorded in the revision 4 delivery report; an earlier full-suite run during concurrent scanner development had 47 passes, two scanner regression failures and one ignored physical test. Physical USB enumeration remains ignored by default. Build warnings about the host's hard-link support do not represent a device operation.
