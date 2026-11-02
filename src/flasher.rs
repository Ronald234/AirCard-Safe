use crate::afc::AfcClient;
use crate::airlift::{
    LINK_PREFIX, RECOVERED_PREFIX, SOURCE_PREFIX, build_books_plist,
    build_streaming_zip_archive_multi, restore_books, snapshot_books, stage_streaming_zip,
};
use crate::device::{
    ActiveDeviceSession, ConnectionMode, DeviceTransport, ensure_transport_available,
};
use crate::{crypto, safety, wallet_backup};
use anyhow::{Context, Result, ensure};
use std::path::Path;

fn write_artwork<L: FnMut(&str)>(
    session: &ActiveDeviceSession,
    afc: &AfcClient,
    card: &str,
    items: &[(&str, &[u8])],
    mut log: L,
) -> Result<()> {
    ensure!(session.transport == DeviceTransport::Usb, "USB is required");
    let target = format!("/var/mobile/Library/Passes/Cards/{card}.pkpass");
    ensure!(items.len() == 3, "All three artwork assets are required");
    for (index, (leaf, data)) in items.iter().enumerate() {
        ensure!(
            safety::valid_target(&target, leaf) && *leaf == safety::ARTWORK[index],
            "Write target is outside the Wallet artwork allowlist"
        );
        ensure!(
            !data.is_empty() && data.len() <= 32 * 1024 * 1024,
            "Invalid artwork payload size"
        );
    }
    ensure_transport_available(&session.udid, DeviceTransport::Usb)?;
    let token = crypto::token()?;
    let source = format!("{SOURCE_PREFIX}{token}");
    let link = format!("{LINK_PREFIX}{token}");
    let recovered = format!("{RECOVERED_PREFIX}{token}");
    let archive = build_streaming_zip_archive_multi(&target, items)?;
    let mut identifiers = vec![format!("../../{source}/p0/p1/p2/link")];
    let mut routes = vec![(identifiers[0].clone(), link.clone())];
    for (index, (leaf, _)) in items.iter().enumerate() {
        let ident = format!("../../{source}/payload_{index}");
        identifiers.push(ident.clone());
        routes.push((ident, format!("{link}/{leaf}")));
    }
    let plist = build_books_plist(&identifiers)?;
    let snapshot = snapshot_books(afc).context("Cannot snapshot Books files; writing blocked")?;
    log("Backup verified. Starting a non-atomic write; do not unplug or close the program.");
    safety::run_with_cleanup(|| {
      (|| -> Result<()> {
        stage_streaming_zip(session, &source, &archive)?;
        ensure!(afc.exists(&format!("{source}/p0/p1/p2/link")), "Staging link is missing");
        for index in 0..items.len() {
            ensure!(afc.exists(&format!("{source}/payload_{index}")), "A staged payload is missing");
        }
        afc.make_directory_recursive("Books/Sync")?;
        afc.write_file("Books/Sync/Books.plist", &plist)?;
        safety::verify_bytes(&plist, &afc.read_file("Books/Sync/Books.plist")?).map_err(anyhow::Error::msg)?;
        let refs: Vec<(&str, &str)> = routes.iter().map(|(a,b)| (a.as_str(), b.as_str())).collect();
        crate::airtraffic::sync_assets_via_airtraffic(&session.udid, DeviceTransport::Usb, &refs, |_| {})?;
        for (leaf, expected) in items {
            let actual = afc.read_file(&format!("{target}/{leaf}"))?;
            safety::verify_bytes(expected, &actual).map_err(anyhow::Error::msg)
                .context("Destination read-back failed. Partial modification is possible; keep the backup.")?;
        }
        Ok(())
      })().map_err(|e| format!("{e:#}"))
    }, || {
    // Always restore tracked Books files, even after a write error.
    restore_books(afc, &snapshot).map_err(|e| format!("{e:#}"))
    }, || {
    let mut cleanup_errors = Vec::new();
    // No recursive traversal: unlink known links before removing empty directories.
    let mut paths = vec![link, recovered, format!("{source}/p0/p1/p2/link")];
    for index in 0..items.len() { paths.push(format!("{source}/payload_{index}")); }
    paths.extend([format!("{source}/payload"), format!("{source}/META-INF/com.apple.ZipMetadata.plist"),
        format!("{source}/META-INF"), format!("{source}/p0/p1/p2"), format!("{source}/p0/p1"), format!("{source}/p0")]);
    let mut nested = String::new();
    let mut dirs = Vec::new();
    for part in target.trim_start_matches('/').split('/') {
        nested.push('/'); nested.push_str(part);
        dirs.push(format!("{source}{nested}"));
    }
    dirs.reverse();
    paths.extend(dirs);
    paths.push(source.clone());
    for path in paths {
        if let Err(e) = afc.remove_path(&path) { cleanup_errors.push(e.to_string()); }
    }
    if cleanup_errors.is_empty() { Ok(()) } else { Err(cleanup_errors.join("; ")) }
    }).map_err(anyhow::Error::msg)?;
    log("Artwork bytes and tracked Books files verified. Visual result remains unverified.");
    Ok(())
}
pub fn apply<L: FnMut(&str)>(
    udid: &str,
    card: &str,
    backup: &Path,
    png: &[u8],
    pdf: &[u8],
    log: L,
) -> Result<()> {
    ensure!(
        crate::scanner::is_valid_card_hash(card),
        "Invalid Wallet card identifier"
    );
    let session = ActiveDeviceSession::open(Some(udid), ConnectionMode::Usb)?;
    let afc = AfcClient::new(&session)?;
    let resolved = wallet_backup::capture_original_card(&afc, backup, udid, card)?;
    let assets = [
        (safety::ARTWORK[0], png),
        (safety::ARTWORK[1], png),
        (safety::ARTWORK[2], pdf),
    ];
    write_artwork(&session, &afc, &resolved, &assets, log)
}
pub fn restore<L: FnMut(&str)>(udid: &str, card: &str, backup: &Path, log: L) -> Result<()> {
    let originals = wallet_backup::load_original_assets(backup, udid, card)?;
    let session = ActiveDeviceSession::open(Some(udid), ConnectionMode::Usb)?;
    let afc = AfcClient::new(&session)?;
    let raw = card.trim_end_matches('=');
    let variants = [card.to_string(), raw.to_string(), format!("{raw}=")];
    let resolved = variants
        .iter()
        .find(|v| afc.exists(&format!("/var/mobile/Library/Passes/Cards/{v}.pkpass")))
        .context("Cannot locate this card over AFC; restore blocked")?;
    let refs: Vec<(&str, &[u8])> = originals
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_slice()))
        .collect();
    write_artwork(&session, &afc, resolved, &refs, log)
}
