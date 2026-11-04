use std::io::{self, Read};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, Ordering},
};
use std::time::{Duration, Instant};

use anyhow::{Result, bail};

use super::{PassKind, WalletItem, extract_wallet_items, has_wallet_context, unified};
use crate::apple::{AMDServiceConnectionRef, AppleLibraries, K_CFPROPERTY_LIST_BINARY_FORMAT_V1_0};
use crate::device::{ActiveDeviceSession, ConnectionMode};
use crate::diagnostics::{DiagnosticSnapshot, LogSource, increment};

#[derive(Default)]
struct Observations(std::collections::HashMap<String, [bool; 7]>);

impl Observations {
    fn accept(&mut self, item: &WalletItem) -> bool {
        let (class, id) = match item {
            WalletItem::PaymentCard(id) => (0, id),
            WalletItem::UnsupportedPass(id) => (1, id),
            WalletItem::CandidatePass(id) => (2, id),
            WalletItem::ClassifiedPass(id, kind) => (
                match kind {
                    PassKind::Generic => 3,
                    PassKind::Loyalty => 4,
                    PassKind::Transit => 5,
                    PassKind::Access => 6,
                },
                id,
            ),
        };
        // Capacity limits new IDs, never stronger evidence for an existing ID.
        // Each ID can emit at most once per class, including read-only revocation.
        if self.0.len() >= 256 && !self.0.contains_key(id) {
            return false;
        }
        let classes = self.0.entry(id.clone()).or_default();
        // Explicit nonpayment evidence is monotonic. A later path/selection
        // cannot downgrade it, even if iOS repeats older events.
        if (class == 0 || class == 2) && (classes[1] || classes[3..].iter().any(|seen| *seen)) {
            return false;
        }
        if class >= 3 && classes[1] {
            return false;
        }
        if class == 3 && classes[4..].iter().any(|seen| *seen) {
            return false;
        }
        let fresh = !classes[class];
        classes[class] = true;
        fresh
    }
}

fn observe_line(
    line: &str,
    seen: &mut Observations,
    diagnostics: &mut DiagnosticSnapshot,
    on_item: &mut impl FnMut(WalletItem),
) {
    increment(&mut diagnostics.lines_received);
    if line.len() > 128 * 1024 {
        increment(&mut diagnostics.oversized_records);
        return;
    }
    if !has_wallet_context(line) {
        return;
    }
    increment(&mut diagnostics.wallet_related);
    let lower = line.to_ascii_lowercase();
    if lower.contains("<private>") || lower.contains("<redacted>") {
        increment(&mut diagnostics.redacted);
    }
    let items = extract_wallet_items(line);
    if items.is_empty() {
        increment(&mut diagnostics.no_identifier);
    }
    for mut item in items {
        if let WalletItem::ClassifiedPass(id, kind) = &item {
            let class = match kind {
                PassKind::Generic => 3,
                PassKind::Loyalty => 4,
                PassKind::Transit => 5,
                PassKind::Access => 6,
            };
            if class >= 4
                && seen.0.get(id).is_some_and(|classes| {
                    classes[4..]
                        .iter()
                        .enumerate()
                        .any(|(index, present)| *present && index + 4 != class)
                })
            {
                item = WalletItem::UnsupportedPass(id.clone());
            }
        }
        if seen.accept(&item) {
            let count = match &item {
                WalletItem::PaymentCard(_) => &mut diagnostics.recognized.payment_candidate,
                WalletItem::CandidatePass(_) => &mut diagnostics.recognized.unknown,
                WalletItem::UnsupportedPass(_) => &mut diagnostics.recognized.unsupported,
                WalletItem::ClassifiedPass(_, kind) => match kind {
                    PassKind::Generic => &mut diagnostics.recognized.generic,
                    PassKind::Loyalty => &mut diagnostics.recognized.loyalty,
                    PassKind::Transit => &mut diagnostics.recognized.transit,
                    PassKind::Access => &mut diagnostics.recognized.access,
                },
            };
            increment(count);
            on_item(item);
        }
    }
}

struct Service<'a> {
    raw: AMDServiceConnectionRef,
    libs: &'a AppleLibraries,
    receive_unavailable: Arc<AtomicU32>,
}

impl Drop for Service<'_> {
    fn drop(&mut self) {
        unsafe {
            (self.libs.amd_service_connection_invalidate)(self.raw);
        }
    }
}

impl Read for Service<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let count = unsafe {
            (self.libs.amd_service_connection_receive)(self.raw, output.as_mut_ptr(), output.len())
        };
        if count < 0 {
            let _ = self.receive_unavailable.fetch_update(
                Ordering::Relaxed,
                Ordering::Relaxed,
                |count| Some(count.saturating_add(1).min(crate::diagnostics::MAX_COUNT)),
            );
            // No verified errno/WSA contract exists for this private secure
            // receive API. Retry without diagnosing the cause as a timeout.
            return Err(io::Error::other(unified::ReceiveUnavailable));
        }
        if count as usize > output.len() {
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok(count as usize)
    }
}

impl<'a> Service<'a> {
    fn open(
        session: &'a ActiveDeviceSession,
        name: &str,
        receive_unavailable: &Arc<AtomicU32>,
    ) -> Result<Self> {
        let raw = session
            .start_service(name)
            .map_err(|_| anyhow::anyhow!("Device log service unavailable"))?;
        let service = Self {
            raw,
            libs: &session.libs,
            receive_unavailable: Arc::clone(receive_unavailable),
        };
        let socket = unsafe { (service.libs.amd_service_connection_get_socket)(raw) };
        if socket <= 0 {
            bail!("Invalid device log connection");
        }
        #[cfg(windows)]
        unsafe {
            let timeout_ms: u32 = 500;
            if super::setsockopt(
                socket as usize,
                super::SOL_SOCKET,
                super::SO_RCVTIMEO,
                &timeout_ms as *const u32 as *const i8,
                std::mem::size_of::<u32>() as i32,
            ) != 0
            {
                bail!("Could not configure scanner timeout");
            }
        }
        Ok(service)
    }

    fn start_unified(&mut self, keep_reading: &mut impl FnMut() -> bool) -> Result<bool> {
        if !keep_reading() {
            return Ok(false);
        }
        let mut options = plist::Dictionary::new();
        options.insert("Request".into(), "StartActivity".into());
        options.insert("Pid".into(), u32::MAX.into());
        options.insert("MessageFilter".into(), 65535u32.into());
        // iOS 27 Windows report #33: 0x13C includes live Wallet paths.
        options.insert("StreamFlags".into(), 0x13Cu32.into());
        let mut bytes = Vec::new();
        plist::Value::Dictionary(options).to_writer_binary(&mut bytes)?;
        let request = self.libs.create_cf_plist_from_bytes(&bytes)?;
        let status = unsafe {
            (self.libs.amd_service_connection_send_message)(
                self.raw,
                request.raw,
                K_CFPROPERTY_LIST_BINARY_FORMAT_V1_0,
            )
        };
        if status != 0 {
            bail!("Device refused log streaming request");
        }
        let Some((kind, payload)) = unified::read_frame(self, keep_reading)? else {
            return Ok(false);
        };
        if kind != 1 {
            bail!("Invalid device log startup reply");
        }
        let reply = plist::Value::from_reader(io::Cursor::new(payload))?;
        if reply
            .as_dictionary()
            .and_then(|dict| dict.get("Status"))
            .and_then(plist::Value::as_string)
            != Some("RequestSuccessful")
        {
            bail!("Device refused log streaming request");
        }
        Ok(true)
    }
}

/// Session-only discovery. Unsupported passes are observations, never write targets.
/// All diagnostics are fixed text; device messages and identifiers are not logged.
pub fn scan_wallet_items<F, L>(
    udid: Option<&str>,
    connection_mode: ConnectionMode,
    stop_flag: Arc<AtomicBool>,
    on_item: F,
    log: L,
) -> Result<()>
where
    F: FnMut(WalletItem),
    L: FnMut(String),
{
    scan_wallet_items_with_diagnostics(udid, connection_mode, stop_flag, on_item, log, |_| {})
}

pub fn scan_wallet_items_with_diagnostics<F, L, D>(
    udid: Option<&str>,
    connection_mode: ConnectionMode,
    stop_flag: Arc<AtomicBool>,
    mut on_item: F,
    mut log: L,
    mut on_diagnostics: D,
) -> Result<()>
where
    F: FnMut(WalletItem),
    L: FnMut(String),
    D: FnMut(DiagnosticSnapshot),
{
    let mut diagnostics = DiagnosticSnapshot::default();
    let receive_unavailable = Arc::new(AtomicU32::new(0));
    let deadline = Instant::now() + Duration::from_secs(120);
    let result = scan_inner(
        udid,
        connection_mode,
        &stop_flag,
        deadline,
        &mut on_item,
        &mut log,
        &mut diagnostics,
        &receive_unavailable,
    );
    diagnostics.receive_unavailable = receive_unavailable.load(Ordering::Relaxed);
    diagnostics.deadline_reached = Instant::now() >= deadline;
    diagnostics.stopped = stop_flag.load(Ordering::Relaxed);
    if result.is_err() {
        diagnostics.stream_failed = true;
        if diagnostics.source == LogSource::None && diagnostics.unavailable_sources == 0 {
            increment(&mut diagnostics.unavailable_sources);
        }
    }
    on_diagnostics(diagnostics);
    result
}

fn scan_inner(
    udid: Option<&str>,
    connection_mode: ConnectionMode,
    stop_flag: &AtomicBool,
    deadline: Instant,
    on_item: &mut impl FnMut(WalletItem),
    log: &mut impl FnMut(String),
    diagnostics: &mut DiagnosticSnapshot,
    receive_unavailable: &Arc<AtomicU32>,
) -> Result<()> {
    let keep_reading = || !stop_flag.load(Ordering::Relaxed) && Instant::now() < deadline;
    if !keep_reading() {
        return Ok(());
    }
    log("Conectando al escáner USB de Wallet...".into());
    let session = ActiveDeviceSession::open(udid, connection_mode).map_err(|_| {
        anyhow::anyhow!(
            "No se pudo abrir una sesión USB verificada. Reconecta y desbloquea el iPhone y acepta la confianza."
        )
    })?;
    if !keep_reading() {
        return Ok(());
    }
    let mut seen = Observations::default();
    let mut service = Service::open(&session, "com.apple.os_trace_relay", receive_unavailable);
    let startup_deadline = (Instant::now() + Duration::from_secs(5)).min(deadline);
    let initialized = match &mut service {
        Ok(service) => {
            service.start_unified(&mut || keep_reading() && Instant::now() < startup_deadline)
        }
        Err(_) => Ok(false),
    };
    if !keep_reading() {
        return Ok(());
    }
    if matches!(initialized, Ok(true)) {
        diagnostics.source = LogSource::Unified;
        log("Registros unificados de Wallet conectados. Abre el pase deseado en Wallet.".into());
        let service = service.as_mut().unwrap();
        while keep_reading() {
            match unified::read_frame(service, &mut || keep_reading()) {
                Ok(Some((2, record))) => {
                    if let Some(line) = unified::record_line(&record) {
                        observe_line(&line, &mut seen, diagnostics, on_item);
                    } else {
                        increment(&mut diagnostics.invalid_frames);
                    }
                }
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(_) => {
                    increment(&mut diagnostics.invalid_frames);
                    bail!(
                        "La lectura de Wallet terminó o recibió un registro inválido. Conecta de nuevo y repite el escaneo."
                    );
                }
            }
        }
    } else {
        increment(&mut diagnostics.unavailable_sources);
        // Startup-only fallback: never change transport or silently restart a live scan.
        drop(service);
        log(
            "Los registros unificados no están disponibles. Probando la fuente USB anterior."
                .into(),
        );
        let mut legacy = Service::open(&session, "com.apple.syslog_relay", receive_unavailable)
            .map_err(|_| {
                increment(&mut diagnostics.unavailable_sources);
                anyhow::anyhow!(
                    "No se pudo iniciar ninguna fuente de registros USB del dispositivo."
                )
            })?;
        diagnostics.source = LogSource::Legacy;
        log("Fuente USB anterior conectada. Abre el pase deseado en Wallet.".into());
        let mut buffer = [0; 8192];
        let mut line = Vec::with_capacity(1024);
        let mut discard = false;
        while keep_reading() {
            match legacy.read(&mut buffer) {
                Ok(0) => bail!(
                    "La fuente de registros de Wallet terminó. Reconecta y repite el escaneo."
                ),
                Ok(count) => {
                    for &byte in &buffer[..count] {
                        if byte == b'\n' || byte == 0 {
                            if !discard && !line.is_empty() {
                                observe_line(
                                    &String::from_utf8_lossy(&line),
                                    &mut seen,
                                    diagnostics,
                                    on_item,
                                );
                            }
                            line.clear();
                            discard = false;
                        } else if !discard && byte != b'\r' {
                            if line.len() < 64 * 1024 {
                                line.push(byte);
                            } else {
                                line.clear();
                                discard = true;
                                increment(&mut diagnostics.oversized_records);
                            }
                        }
                    }
                }
                Err(error) if unified::should_retry_receive(&error) => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                Err(_) => bail!("No se pudo continuar la lectura de registros de Wallet."),
            }
        }
    }
    log("Escaneo de Wallet detenido.".into());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_read_only_revocation_is_emitted_after_distinct_id_capacity() {
        let mut observations = Observations::default();
        for index in 0..256 {
            assert!(
                observations.accept(&WalletItem::PaymentCard(format!("synthetic-pass-{index}")))
            );
        }
        assert!(observations.accept(&WalletItem::UnsupportedPass("synthetic-pass-0".into())));
        assert!(!observations.accept(&WalletItem::UnsupportedPass("synthetic-pass-0".into())));
        assert!(!observations.accept(&WalletItem::PaymentCard("synthetic-pass-256".into())));
        assert!(!observations.accept(&WalletItem::UnsupportedPass("synthetic-pass-256".into())));
        assert!(!observations.accept(&WalletItem::PaymentCard("synthetic-pass-0".into())));
    }

    #[test]
    fn existing_candidate_accepts_both_evidence_upgrades_at_capacity() {
        let mut observations = Observations::default();
        for index in 0..256 {
            assert!(observations.accept(&WalletItem::CandidatePass(format!(
                "synthetic-pass-{index}"
            ))));
        }
        assert!(observations.accept(&WalletItem::PaymentCard("synthetic-pass-0".into())));
        assert!(observations.accept(&WalletItem::UnsupportedPass("synthetic-pass-0".into())));
        for item in [
            WalletItem::CandidatePass("synthetic-pass-0".into()),
            WalletItem::PaymentCard("synthetic-pass-0".into()),
            WalletItem::UnsupportedPass("synthetic-pass-0".into()),
            WalletItem::CandidatePass("synthetic-pass-256".into()),
        ] {
            assert!(!observations.accept(&item));
        }
    }
}

#[cfg(test)]
mod revision4_tests {
    use super::*;
    #[test]
    fn secret_raw_events_are_counted_without_entering_diagnostic_export() {
        let mut snapshot = crate::diagnostics::DiagnosticSnapshot::default();
        let mut seen = Observations::default();
        let mut items = Vec::new();
        for line in [
            "passd: selected pass uniqueID : <private> token=SECRET_TOKEN account=SECRET_ACCOUNT",
            "webserver: credential=SECRET_KEY wallet",
            "Passbook: Dashboard loading (...): for synthetic-pass-123456789, secret=SECRET_TOKEN",
        ] {
            observe_line(line, &mut seen, &mut snapshot, &mut |item| items.push(item));
        }
        let json = crate::diagnostics::export_json(&snapshot).unwrap();
        for secret in [
            "SECRET_TOKEN",
            "SECRET_ACCOUNT",
            "SECRET_KEY",
            "synthetic-pass-123456789",
            "passd:",
            "Dashboard",
        ] {
            assert!(!json.contains(secret));
        }
        assert_eq!(snapshot.lines_received, 3);
        assert_eq!(snapshot.wallet_related, 2);
        assert_eq!(snapshot.redacted, 1);
        assert_eq!(snapshot.no_identifier, 1);
        assert_eq!(snapshot.recognized.unknown, 1);
        assert_eq!(items.len(), 1);
    }
    #[test]
    fn cancelled_scan_produces_final_diagnostics_without_opening_a_session() {
        let mut final_snapshot = None;
        scan_wallet_items_with_diagnostics(
            None,
            ConnectionMode::Usb,
            Arc::new(AtomicBool::new(true)),
            |_| panic!("no item"),
            |_| {},
            |snapshot| final_snapshot = Some(snapshot),
        )
        .unwrap();
        assert!(final_snapshot.unwrap().stopped);
    }
}

#[cfg(test)]
mod evidence_tests {
    use super::*;
    #[test]
    fn repeated_events_and_specific_conflicts_cannot_erase_read_only_evidence() {
        let mut diagnostics = DiagnosticSnapshot::default();
        let mut seen = Observations::default();
        let mut emitted = Vec::new();
        let id = "4FD0Y4djT-oIj--0Odq8GMLVpVo";
        for metadata in [
            "passType: generic;",
            "passType: loyalty;",
            "",
            "passType: loyalty;",
            "passType: access;",
            "passType: transit;",
            "",
        ] {
            observe_line(
                &format!("nfcd: {metadata} selected pass uniqueID : \"{id}\""),
                &mut seen,
                &mut diagnostics,
                &mut |item| emitted.push(item),
            );
        }
        assert_eq!(
            emitted,
            vec![
                WalletItem::ClassifiedPass(id.into(), PassKind::Generic),
                WalletItem::ClassifiedPass(id.into(), PassKind::Loyalty),
                WalletItem::UnsupportedPass(id.into())
            ]
        );
        assert_eq!(diagnostics.recognized.generic, 1);
        assert_eq!(diagnostics.recognized.loyalty, 1);
        assert_eq!(diagnostics.recognized.unsupported, 1);
    }
}
