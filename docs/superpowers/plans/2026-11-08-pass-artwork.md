# Pass artwork inventory implementation plan

Goal: remove the payment-background-only assumption in an isolated resource-selection component. This does not edit a phone or authorize writing.

Architecture: parse bounded caller-supplied metadata into one explicit pass layout and an NFC-presence flag; discard credentials and identifiers. Select only exact standard PNG resource names present in a same-pass, root-only inventory. Unknown layouts, conflicting/duplicate metadata and path ambiguity fail closed. The caller must establish provenance; this module cannot.

Files: research/recovery-store/src/pass_artwork.rs; research/recovery-store/tests/pass_artwork.rs; .github/workflows/pass-artwork.yml.

- [ ] Establish failing behavioral tests on Windows and Linux.
- [ ] Implement bounded metadata parsing with duplicate rejection, fixed errors and no sensitive output.
- [ ] Implement layout-specific selection from existing filenames, rejecting traversal, aliases and absent resources.
- [ ] Run the complete research suite, formatting, strict Clippy and release compilation on both platforms.

Constraints: no phone calls, no NFC payload modification, no metadata/signature write targets, no pass-id guessing. No new dependencies. No claim of phone editing or NFC reader compatibility. PNG names are resource candidates, not a guarantee of visible rendering. Localized resources and modern poster layouts remain unsupported. A modified signed package would need valid issuer signing before import; this component neither signs nor installs packages.

Recovery integration is separate: the reviewed Linux engine accepts only payment background names and uses a Linux-native adapter. It is not a drop-in Windows NFC-pass editor. Reading actual images, preserving all preimages and physically validating interrupted recovery remain necessary.
