//! Allowlisted aggregate diagnostics: this type cannot hold raw device text.
use serde::Serialize;

pub const MAX_COUNT: u32 = 1_000_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogSource {
    #[default]
    None,
    Unified,
    Legacy,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct RecognizedKinds {
    pub payment_candidate: u32,
    pub unknown: u32,
    pub generic: u32,
    pub loyalty: u32,
    pub transit: u32,
    pub access: u32,
    pub unsupported: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct DiagnosticSnapshot {
    pub lines_received: u32,
    pub wallet_related: u32,
    pub redacted: u32,
    pub no_identifier: u32,
    pub unmatched_events: UnmatchedEvents,
    pub recognized: RecognizedKinds,
    pub source: LogSource,
    pub unavailable_sources: u32,
    pub invalid_frames: u32,
    pub receive_unavailable: u32,
    pub oversized_records: u32,
    pub deadline_reached: bool,
    pub stopped: bool,
    pub stream_failed: bool,
}

/// Mutually exclusive reasons for events from which the parser extracted no item.
/// A redaction marker anywhere in an event does not prove its identifier was redacted.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct UnmatchedEvents {
    pub redaction_marker_present: u32,
    pub known_shape_without_item: u32,
    pub other_shape: u32,
}

impl DiagnosticSnapshot {
    pub(crate) fn record_unmatched(&mut self, redacted: bool, known_shape: bool) {
        let count = if redacted {
            &mut self.unmatched_events.redaction_marker_present
        } else if known_shape {
            &mut self.unmatched_events.known_shape_without_item
        } else {
            &mut self.unmatched_events.other_shape
        };
        increment(count);
    }
}

pub(crate) fn increment(count: &mut u32) {
    *count = count.saturating_add(1).min(MAX_COUNT);
}

impl DiagnosticSnapshot {
    pub fn explanation(&self) -> String {
        let observation = self.observation_explanation();
        if self.receive_unavailable > 0 && !self.stream_failed && self.invalid_frames == 0 {
            format!(
                "{observation} Hubo reintentos de recepción: por sí solos no prueban pérdida de eventos ni una desconexión. Tampoco garantizan que la captura sea completa."
            )
        } else {
            observation.into()
        }
    }

    fn observation_explanation(&self) -> &'static str {
        if self.source == LogSource::None && self.unavailable_sources > 0 {
            "La fuente de registros no estuvo disponible. No se puede determinar el estado del pase."
        } else if self.stream_failed || self.invalid_frames > 0 {
            "La lectura de registros quedó incompleta. Repite el escaneo; esto no demuestra compatibilidad del pase."
        } else if self.wallet_related == 0 {
            "No se observaron eventos de Wallet. Esto no demuestra que el pase esté ausente."
        } else if self.unmatched_events.known_shape_without_item > 0 {
            "Hay eventos con patrones conocidos de Wallet de los que el detector no extrajo un identificador utilizable. Puede faltar un dato o no coincidir el formato esperado; no demuestra que el pase esté ausente ni permite habilitar su edición."
        } else if self.redacted > 0 && self.no_identifier > 0 {
            "Se observaron datos ocultos y eventos sin identificador utilizable. Los datos privados no se recuperan."
        } else if self.no_identifier > 0 {
            "Algunos eventos de Wallet no tenían identificador utilizable. Esos eventos no permiten determinar el tipo de pase."
        } else if [
            self.recognized.generic,
            self.recognized.loyalty,
            self.recognized.transit,
            self.recognized.access,
            self.recognized.unsupported,
        ]
        .iter()
        .any(|count| *count > 0)
        {
            "Se observó un tipo de pase explícito o un contexto de imagen incompatible. Sigue en modo de solo lectura; detectarlo no demuestra compatibilidad."
        } else {
            "Se observaron candidatos de Wallet. Un identificador o una ruta de caché no confirma funciones de pago, NFC, código de barras o acceso."
        }
    }
}

/// Only a closed aggregate schema can enter the exporter. No arbitrary payloads.
pub fn export_json(snapshot: &DiagnosticSnapshot) -> serde_json::Result<String> {
    let mut safe = snapshot.clone();
    for count in [
        &mut safe.lines_received,
        &mut safe.wallet_related,
        &mut safe.redacted,
        &mut safe.no_identifier,
        &mut safe.unmatched_events.redaction_marker_present,
        &mut safe.unmatched_events.known_shape_without_item,
        &mut safe.unmatched_events.other_shape,
        &mut safe.unavailable_sources,
        &mut safe.invalid_frames,
        &mut safe.oversized_records,
        &mut safe.recognized.payment_candidate,
        &mut safe.receive_unavailable,
        &mut safe.recognized.unknown,
        &mut safe.recognized.generic,
        &mut safe.recognized.loyalty,
        &mut safe.recognized.transit,
        &mut safe.recognized.access,
        &mut safe.recognized.unsupported,
    ] {
        *count = (*count).min(MAX_COUNT);
    }
    serde_json::to_string_pretty(&safe)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unmatched_reasons_are_exclusive_bounded_and_do_not_claim_private_id() {
        let mut snapshot = DiagnosticSnapshot::default();
        snapshot.record_unmatched(true, true);
        snapshot.record_unmatched(false, true);
        snapshot.record_unmatched(false, false);
        assert_eq!(snapshot.unmatched_events.redaction_marker_present, 1);
        assert_eq!(snapshot.unmatched_events.known_shape_without_item, 1);
        assert_eq!(snapshot.unmatched_events.other_shape, 1);
        snapshot.unmatched_events.other_shape = u32::MAX;
        let value: serde_json::Value =
            serde_json::from_str(&export_json(&snapshot).unwrap()).unwrap();
        assert_eq!(value["unmatched_events"]["other_shape"], MAX_COUNT);
    }

    #[test]
    fn diagnostics_export_accepts_only_closed_aggregate_fields_and_bounds_counts() {
        let snapshot = DiagnosticSnapshot {
            lines_received: u32::MAX,
            ..Default::default()
        };
        let json = export_json(&snapshot).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["lines_received"], 1_000_000);
        assert_eq!(value.as_object().unwrap().len(), 14);
        assert!(value.get("raw").is_none());
    }
    #[test]
    fn explanations_distinguish_absence_redaction_and_unavailable_source() {
        let mut snapshot = DiagnosticSnapshot::default();
        assert!(snapshot.explanation().contains("No se observaron eventos"));
        snapshot.lines_received = 1;
        snapshot.wallet_related = 1;
        snapshot.no_identifier = 1;
        assert!(snapshot.explanation().contains("identificador utilizable"));
        snapshot.redacted = 1;
        assert!(snapshot.explanation().contains("datos ocultos"));
        snapshot.unavailable_sources = 1;
        assert!(snapshot.explanation().contains("no estuvo disponible"));
    }
}

#[cfg(test)]
mod unavailable_receive_tests {
    use super::*;
    #[test]
    fn retryable_receive_results_do_not_prove_missing_events() {
        let snapshot = DiagnosticSnapshot {
            receive_unavailable: 4,
            source: LogSource::Unified,
            ..Default::default()
        };
        assert!(!snapshot.explanation().contains("incompleta"));
        assert!(snapshot.explanation().contains("reintentos"));
        assert!(!snapshot.explanation().contains("se desconectó"));
        assert!(
            export_json(&snapshot)
                .unwrap()
                .contains("\"receive_unavailable\": 4")
        );
    }

    #[test]
    fn parser_evidence_is_not_hidden_by_redaction_or_receive_retries() {
        let mut snapshot = DiagnosticSnapshot {
            wallet_related: 10,
            no_identifier: 8,
            redacted: 2,
            receive_unavailable: 1,
            source: LogSource::Unified,
            unmatched_events: UnmatchedEvents {
                known_shape_without_item: 3,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(snapshot.explanation().contains("patrones conocidos"));
        assert!(snapshot.explanation().contains("reintentos"));
        snapshot.stream_failed = true;
        assert!(snapshot.explanation().contains("incompleta"));
        snapshot.stream_failed = false;
        snapshot.invalid_frames = 1;
        assert!(snapshot.explanation().contains("incompleta"));
    }
}
