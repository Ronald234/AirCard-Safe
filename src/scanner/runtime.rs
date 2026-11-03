use std::io::{self, Read};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use anyhow::{Result, bail};

use super::{WalletItem, extract_wallet_items, unified};
use crate::apple::{AMDServiceConnectionRef, AppleLibraries, K_CFPROPERTY_LIST_BINARY_FORMAT_V1_0};
use crate::device::{ActiveDeviceSession, ConnectionMode};

#[derive(Default)]
struct Observations(std::collections::HashMap<String, [bool; 3]>);

impl Observations {
    fn accept(&mut self, item: &WalletItem) -> bool {
        let (class, id) = match item {
            WalletItem::PaymentCard(id) => (0, id),
            WalletItem::UnsupportedPass(id) => (1, id),
            WalletItem::CandidatePass(id) => (2, id),
        };
        // Capacity limits new IDs, never stronger evidence for an existing ID.
        // Each ID can emit at most once per class, including read-only revocation.
        if self.0.len() >= 256 && !self.0.contains_key(id) {
            return false;
        }
        let classes = self.0.entry(id.clone()).or_default();
        let fresh = !classes[class];
        classes[class] = true;
        fresh
    }
}

struct Service<'a> {
    raw: AMDServiceConnectionRef,
    libs: &'a AppleLibraries,
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
    fn open(session: &'a ActiveDeviceSession, name: &str) -> Result<Self> {
        let raw = session
            .start_service(name)
            .map_err(|_| anyhow::anyhow!("Device log service unavailable"))?;
        let service = Self {
            raw,
            libs: &session.libs,
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
    mut on_item: F,
    mut log: L,
) -> Result<()>
where
    F: FnMut(WalletItem),
    L: FnMut(String),
{
    let deadline = Instant::now() + Duration::from_secs(120);
    let keep_reading = || !stop_flag.load(Ordering::Relaxed) && Instant::now() < deadline;
    if !keep_reading() {
        return Ok(());
    }
    log("Connecting to the USB Wallet scanner...".into());
    let session = ActiveDeviceSession::open(udid, connection_mode).map_err(|_| {
        anyhow::anyhow!(
            "Could not open a verified USB device session. Reconnect, unlock and trust the iPhone."
        )
    })?;
    if !keep_reading() {
        return Ok(());
    }
    let mut seen = Observations::default();
    let mut emit = |line: &str| {
        for item in extract_wallet_items(line) {
            // Bound retained candidates as well as the frame buffers.
            if seen.accept(&item) {
                on_item(item);
            }
        }
    };
    let mut service = Service::open(&session, "com.apple.os_trace_relay");
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
        log("Unified Wallet scanner connected. Open the desired card in Wallet.".into());
        let service = service.as_mut().unwrap();
        while keep_reading() {
            match unified::read_frame(service, &mut || keep_reading()) {
                Ok(Some((2, record))) => {
                    if let Some(line) = unified::record_line(&record) {
                        emit(&line);
                    }
                }
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(_) => bail!(
                    "Unified Wallet log stream ended or returned an invalid frame. Reconnect and scan again."
                ),
            }
        }
    } else {
        // Startup-only fallback: never change transport or silently restart a live scan.
        drop(service);
        log("Unified logs unavailable. Trying the legacy USB Wallet scanner.".into());
        let mut legacy = Service::open(&session, "com.apple.syslog_relay")
            .map_err(|_| anyhow::anyhow!("Neither USB device log service could be started."))?;
        log("Legacy scanner connected. Open the desired card in Wallet.".into());
        let mut buffer = [0; 8192];
        let mut line = Vec::with_capacity(1024);
        let mut discard = false;
        while keep_reading() {
            match legacy.read(&mut buffer) {
                Ok(0) => bail!("Legacy Wallet log stream ended. Reconnect and scan again."),
                Ok(count) => {
                    for &byte in &buffer[..count] {
                        if byte == b'\n' || byte == 0 {
                            if !discard && !line.is_empty() {
                                emit(&String::from_utf8_lossy(&line));
                            }
                            line.clear();
                            discard = false;
                        } else if !discard && byte != b'\r' {
                            if line.len() < 64 * 1024 {
                                line.push(byte);
                            } else {
                                line.clear();
                                discard = true;
                            }
                        }
                    }
                }
                Err(error) if unified::should_retry_receive(&error) => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                Err(_) => bail!("Legacy Wallet log stream failed."),
            }
        }
    }
    log("Wallet scan stopped.".into());
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
