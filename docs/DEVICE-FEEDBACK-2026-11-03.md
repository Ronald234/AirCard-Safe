# Reported device result: no visible artwork change

Follow-up: the original Mac README at `52ba05dca4bdc590672c19d554701f4438e20338` explicitly excludes Apple Card's dynamic vector-rendered face from static cached artwork. The current refresh/discovery corrections do not establish Apple Card support. See REFRESH-FIX.md. This report remains a record of the earlier failed user test.

## Evidence and its limits

- The user reports applying artwork to Apple Card without a visible change.
- The user reports that existing gym/membership passes in Wallet are not detected.
- The live Windows UI showed the current original-method build's unverified-transfer completion message. Its device selector showed `iPhone17,1`, iOS `27.0`.
- This is a user-reported phone result plus an observed Windows UI state. The assistant did not inspect the phone screen, verify its protected files, perform an additional write or test payment/NFC operation.

The reported outcome is a failed visual customization test. It must not be recorded as successful iPhone compatibility just because the Windows tests/build and transfer return value succeeded.

## Confirmed source limitations

1. The scanner accepts a narrow set of 20/32-byte cryptographic card identifier formats. It is not a general Wallet membership-pass enumerator. The writer only supports three bank-style artwork filenames under the Cards directory. Broadening a regular expression alone would not implement safe membership-pass customization.
2. Upstream calls cache invalidation after artwork synchronization, writing invalid bytes to six cache leaves. That step was deliberately removed from this build. It can affect visible refresh, but its omission has not been proven to be the sole cause on this phone.
3. The current AirTraffic helper checks a Book asset manifest, dispatches completion messages, waits two seconds and returns. This does not verify protected destination bytes or a Wallet refresh. The application correctly labels the result unverified.

## Reference observations

- [Upstream source](https://github.com/Lumid-Off/AirCard-Windows/blob/88131e301c88fd9238cbeff9d091967c3ddbf10f/src/flasher.rs): cache invalidation is part of the original visual-update sequence.
- [Upstream issue 33](https://github.com/Lumid-Off/AirCard-Windows/issues/33): a contributor reports iOS 27.0 discovery through unified logs, with artwork/cache operations. This is not this project's physical validation and does not establish gym-pass compatibility.
- [Upstream issue 64](https://github.com/Lumid-Off/AirCard-Windows/issues/64) and [issue 75](https://github.com/Lumid-Off/AirCard-Windows/issues/75): reports of completed operations without a visible result on other iOS versions, including a report with cache invalidation. Those reports do not prove the cause on this user's iOS 27.0 phone.

## Pending

Identify the gym pass type before changing the detector or write allowlist. There is no confirmed supported artwork target or verified restoration method for that pass. Do not claim a corrected gym-compatible executable or diagnose a definitive Apple Card cause without further evidence. No additional cache/protected-file modification was performed during this diagnosis.
