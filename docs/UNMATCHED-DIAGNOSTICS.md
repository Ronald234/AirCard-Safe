# Unmatched Wallet diagnostics experiment

The JSON now contains `unmatched_events` with three mutually exclusive, capped numeric counters. `redaction_marker_present` means the unmatched event contained a privacy marker somewhere; it does not prove the ID was hidden. `known_shape_without_item` means a recognized structural marker was present but the parser extracted no item. `other_shape` covers remaining unmatched Wallet events. Counters do not store raw messages, identifiers, filenames or NFC payloads.

These observations do not determine whether a pass is compatible or whether its images are recoverable. `payment_candidate` remains a heuristic, not a confirmed bank-card classification. No credential or device-writing behavior changes.

For a new experiment: connect by USB, start a fresh scan, open only the intended pass directly in Wallet, stop the scan and export a new JSON. No purchase or door-reader transaction is needed for this observation attempt. Zero useful events remains a possible outcome. Do not publish raw phone logs.

Validation: the aggregate regression was observed failing before implementation, then all four isolated diagnostics tests passed locally. The full app cannot resolve uncached dependencies offline on the local host; Windows CI must validate runtime integration, full regression and executable build. Artifact is a diagnostic experiment, not a fix enabling editing.
