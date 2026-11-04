# Security review — revision 4

## Scope and threat model

Static source review plus automated tests; not a penetration test or physical-device certification. Inputs include untrusted image bytes, malformed/fragmented device logs, opaque identifiers, local backup manifests, Windows runtime discovery and output-file choices. Private Apple APIs and their ABI remain a platform risk.

## Controls

- Device apply and restore entry points fail before device connection, staging, Books sync or cleanup. UI acknowledgement and locally authored SHA-256 manifests cannot unlock them.
- Archival AFC/StreamingZip/AirTraffic writer implementations are compiled for tests only; they are not linked into the release application. No keypad module exists.
- Discovery requires USB and rejects ambiguous network advertisements. No Wi-Fi fallback.
- Log framing/record/line/identifier limits bound hostile input. Raw logs and opaque IDs remain in process memory only; no names/IDs are exported. Rust strings are not securely zeroized; process memory/crash dumps remain a local privacy risk.
- Export schema contains only closed enum states and bounded counters. It cannot accept arbitrary log text. Sentinel tests exercise secrets in input and check the complete serialized output.
- Local export creates a new file; existing files are not truncated. A failed write can leave a partial new file, which is reported. Image ZIPs are designs, not backups or signed passes.
- Image decode is limited to 20 MiB input, 8192 pixels per dimension and a 128 MiB allocation limit. Generated PNG/PDF files use fixed archive names.
- MSIX/DLL search and AirTraffic protocol risks are documented in TRANSPORT-AUDIT.md. No Apple proprietary DLL is redistributed.
- Actions has read-only repository permissions, pinned action SHAs, locked Cargo resolution, explicit process exit checks, SHA-256 generation and PE/self-check/GUI smoke gates.

## Rejected unsafe behavior

The fork's move-based backup, partial backup acceptance, delete-before-rename, blanket staging cleanup, unverified cache replacement and detached timeout are not accepted as a recovery method. Restoring Media/Books metadata alone does not prove restoration of protected Wallet artwork. See RECOVERY-AUDIT.md for source references and failure sequences.

## Dependency and remaining risks

The targeted advisory review found the inherited `zip 2.4.2` in affected version ranges for extraction advisories [RUSTSEC-2026-0367](https://rustsec.org/advisories/RUSTSEC-2026-0367.html), [0368](https://rustsec.org/advisories/RUSTSEC-2026-0368.html), [0369](https://rustsec.org/advisories/RUSTSEC-2026-0369.html) and [0429](https://rustsec.org/advisories/RUSTSEC-2026-0429.html). These concern archive extraction, symlinks/permissions and allocation, with fixes listed from 9.0.0. Revision 4 upgrades to `zip 9.3.0` with default features disabled and updates Cargo.lock. The application exports fixed-name archives and offers no archive import/extraction flow; this finding is dependency maintenance, not a demonstrated exploit in the released application. The full locked suite and release build must be repeated after the update.

Clippy correctness analysis and a dependency-tree inventory are included in CI. Cargo.lock pins resolution; it does not prove dependencies are vulnerability-free. A full fresh RustSec advisory audit and independent native ABI review remain pending unless separately recorded in STATUS.md. Do not interpret this review as a malware-free guarantee.

Some native secure-receive calls may ignore the socket deadline; the application can request cancellation but cannot guarantee an absolute timeout. A build/self-check does not exercise those calls on a real phone. Authentication, banking data, NFC keys and access credentials are never requested or modified by this revision.

The executable is unsigned. Use its artifact checksum for integrity, and download from the repository's actual successful workflow run. This does not establish publisher identity or eliminate antivirus false positives.

## Supplementary audit execution block

The supplementary cargo-audit 0.22.2 attempt could not be verified complete. Initial tool compilation failed on temporary-output access; a retry used a project-local build directory, but the command environment subsequently rejected an invalid Linux-style sandbox URI on Windows. This remains a documented verification gap. Full locked tests, Clippy correctness analysis, release compilation, PE checks and GUI smoke checks passed in Windows Actions. Exact results and executable SHA-256 are in STATUS.md.
