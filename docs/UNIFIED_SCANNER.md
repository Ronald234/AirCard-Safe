> Revision 4: see SCANNER-DIAGNOSTICS.md for current classifications and diagnostics. All device mutations are blocked. The MIT attribution below remains applicable.

# Native Wallet discovery references and limits

`src/scanner/unified.rs` and `src/scanner/runtime.rs` adapt the unified-log
framing and activity-record decoding approach from Mak5er/AirCard:

- Commit `52ba05dca4bdc590672c19d554701f4438e20338`.
- `Sources/os_trace.h`, `Sources/device_helper.m`, and
  `Sources/WalletDiscovery.swift`.
- https://github.com/Mak5er/AirCard/tree/52ba05dca4bdc590672c19d554701f4438e20338
- Thanks to Mak5er, Johnny Franks, and the AirCard contributors.

The `0x13C` stream option follows the single-device iOS 27 Windows report:
https://github.com/Lumid-Off/AirCard-Windows/issues/33
That report is evidence for a discovery route, not proof of all-pass support.

The public libimobiledevice `include/libimobiledevice/ostrace.h` and
`src/ostrace.c` were consulted for protocol field definitions and startup
semantics; their implementation was not incorporated.

## Behavior and limits

The scanner uses the installed Apple Mobile Device Support APIs directly.
No Python runtime, extra helper, network route or dependency installation is
required. The unified-log reply wait has a five-second limit; only failed or
unavailable initialization permits a legacy USB syslog fallback. The receive
loops stop after 120 seconds or a cooperative cancellation. Service handles are
invalidated on every return path. Socket receive timeout is 500 milliseconds;
the native Apple API determines how that timeout is honored on a secure
connection. Synchronous session/service startup calls have no separately proven
cancellation bound.

`AMDServiceConnectionReceive` exposes a signed byte count here. The consulted
primary source usages do not establish a Windows secure-connection contract for
negative return values or reliable `errno`/`WSAGetLastError` propagation. A
negative return is therefore an opaque retry signal, not a diagnosed timeout or
disconnect. Retries sleep for 10 milliseconds and remain limited by cancellation
and the receive-loop deadlines. Zero-byte EOF and malformed frames are handled
separately. A persistent negative return can keep an initialized scanner waiting
until cancellation or its deadline; it is not reported as a confirmed timeout.

Buffers are bounded: frames 16 MiB, decoded messages 64 KiB, legacy lines
64 KiB, and retained distinct identifiers 256, with at most seven classification emissions per identifier (1792 total). Partial frames survive retryable receive results; native negative-result causes remain unknown.
Malformed lengths, unknown frames, truncated records and invalid slices are
rejected. Diagnostics contain fixed status text, never raw device messages,
pass names, identifiers, pairing data or credentials.

Concrete Cards resource paths with accepted SHA-style IDs retain the legacy
payment-card classification. Explicit membership context, incompatible layout,
or an incompatible path ID produces `UnsupportedPass`. An untyped selected-pass,
library-write or NFC in-session observation produces `CandidatePass`; this
observation alone cannot be written. Both are session-only. Callers merge
observations by exact ID with precedence `UnsupportedPass` over `PaymentCard`
over `CandidatePass`, so explicit incompatible evidence is never upgraded by a
later path. An opaque hash by itself is not proof of a payment card or of artwork
compatibility.
`passIDs[global]` is Express Mode configuration and does not produce an item.
Privacy-redacted fields cannot be reconstructed. No Mac Wallet cache is treated
as a Windows phone inventory. A successful scanner does not establish that a
membership or NFC credential can use the payment-style artwork assets.

Validation uses synthetic byte streams and identifiers only. No real device
logs were collected and no physical-device behavior was tested for this change.

## Upstream MIT notice

MIT License

Copyright (c) 2026 Johnny Franks

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

The UI merges observations by identifier. Explicit incompatible evidence always takes precedence over inferred payment compatibility, including after the runtime reaches its identifier cap. Untyped observations may be corroborated by a concrete Cards path but never authorize writes by themselves. Explicit Apple Card context is incompatible with this static-artwork method.
