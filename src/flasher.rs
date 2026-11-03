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

fn write_batch<L: FnMut(&str)>(
    session: &ActiveDeviceSession,
    afc: &AfcClient,
    card: &str,
    kind: safety::WalletBatchKind,
    items: &[(&str, &[u8])],
    mut log: L,
) -> Result<()> {
    ensure!(session.transport == DeviceTransport::Usb, "USB is required");
    let target = match kind {
        safety::WalletBatchKind::Artwork => safety::wallet_batch_target(card, items),
        _ => safety::wallet_cache_target(card, kind, items),
    }
    .map_err(anyhow::Error::msg)?;
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
    log(
        "Original transfer method. No verified Wallet backup or destination readback is available. Do not unplug.",
    );
    safety::run_with_cleanup(
        || {
            (|| -> Result<()> {
                stage_streaming_zip(session, &source, &archive)?;
                ensure!(
                    afc.try_exists(&format!("{source}/p0/p1/p2/link"))?,
                    "Staging link is missing"
                );
                let staged_paths: Vec<_> = (0..items.len())
                    .map(|i| format!("{source}/payload_{i}"))
                    .collect();
                let staged: Vec<_> = staged_paths
                    .iter()
                    .zip(items)
                    .map(|(path, (_, bytes))| (path.as_str(), *bytes))
                    .collect();
                safety::verify_staged_payloads(&staged, |path| {
                    afc.read_file(path).map_err(|e| e.to_string())
                })
                .map_err(anyhow::Error::msg)?;
                afc.make_directory_recursive("Books/Sync")?;
                afc.write_file("Books/Sync/Books.plist", &plist)?;
                safety::verify_bytes(&plist, &afc.read_file("Books/Sync/Books.plist")?)
                    .map_err(anyhow::Error::msg)?;
                let refs: Vec<(&str, &str)> = routes
                    .iter()
                    .map(|(a, b)| (a.as_str(), b.as_str()))
                    .collect();
                crate::airtraffic::sync_assets_via_airtraffic(
                    &session.udid,
                    DeviceTransport::Usb,
                    &refs,
                    |_| {},
                )?;
                // As upstream, a completed sync is a submission, not proof of a visual
                // change or protected-file readback. Ordinary AFC is rooted in Media.
                Ok(())
            })()
            .map_err(|e| format!("{e:#}"))
        },
        || {
            // Always restore tracked Books files, even after a write error.
            restore_books(afc, &snapshot).map_err(|e| format!("{e:#}"))
        },
        || {
            let mut cleanup_errors = Vec::new();
            // No recursive traversal: unlink known links before removing empty directories.
            let mut paths = vec![link, recovered, format!("{source}/p0/p1/p2/link")];
            for index in 0..items.len() {
                paths.push(format!("{source}/payload_{index}"));
            }
            paths.extend([
                format!("{source}/payload"),
                format!("{source}/META-INF/com.apple.ZipMetadata.plist"),
                format!("{source}/META-INF"),
                format!("{source}/p0/p1/p2"),
                format!("{source}/p0/p1"),
                format!("{source}/p0"),
            ]);
            let mut nested = String::new();
            let mut dirs = Vec::new();
            for part in target.trim_start_matches('/').split('/') {
                nested.push('/');
                nested.push_str(part);
                dirs.push(format!("{source}{nested}"));
            }
            dirs.reverse();
            paths.extend(dirs);
            paths.push(source.clone());
            for path in paths {
                if let Err(e) = afc.remove_path(&path) {
                    cleanup_errors.push(e.to_string());
                }
            }
            if cleanup_errors.is_empty() {
                Ok(())
            } else {
                Err(cleanup_errors.join("; "))
            }
        },
    )
    .map_err(anyhow::Error::msg)?;
    log(
        "Transfer submitted; tracked Books files restored. Wallet bytes, appearance and rollback remain unverified.",
    );
    Ok(())
}
fn transfer_with_refresh<L: FnMut(&str)>(
    session: &ActiveDeviceSession,
    afc: &AfcClient,
    card: &str,
    artwork: &[(&str, &[u8])],
    mut log: L,
) -> Result<()> {
    // Restore upstream's artwork -> .cache -> .pkcache order. Unlike upstream,
    // stop on the first failed phase, including restore/cleanup failures.
    // Never retry a possibly partially completed batch as individual files.
    let caches = safety::CACHE_LEAVES.map(|leaf| (leaf, safety::CACHE_INVALIDATION_BYTES));
    safety::run_wallet_sequence(|kind| {
        let (message, items): (&str, &[(&str, &[u8])]) = match kind {
            safety::WalletBatchKind::Artwork => {
                ("Paso 1/3: enviando imagenes de la tarjeta.", artwork)
            }
            safety::WalletBatchKind::Cache => (
                "Paso 2/3: solicitando refresco de la cache de Wallet.",
                &caches,
            ),
            safety::WalletBatchKind::PkCache => (
                "Paso 3/3: solicitando refresco de la segunda cache de Wallet.",
                &caches,
            ),
        };
        log(message);
        write_batch(session, afc, card, kind, items, |_| {}).map_err(|e| format!("{e:#}"))
    })
    .map_err(anyhow::Error::msg)
}

pub fn apply<L: FnMut(&str)>(
    udid: &str,
    card: &str,
    png: &[u8],
    png2x: &[u8],
    pdf: &[u8],
    log: L,
) -> Result<()> {
    ensure!(
        crate::scanner::is_valid_card_hash(card),
        "Invalid Wallet card identifier"
    );
    let session = ActiveDeviceSession::open(Some(udid), ConnectionMode::Usb)?;
    let afc = AfcClient::new(&session)?;
    let assets = [
        (safety::ARTWORK[0], png),
        (safety::ARTWORK[1], png2x),
        (safety::ARTWORK[2], pdf),
    ];
    transfer_with_refresh(&session, &afc, card, &assets, log)
}
pub fn restore<L: FnMut(&str)>(udid: &str, card: &str, backup: &Path, log: L) -> Result<()> {
    let originals = wallet_backup::load_original_assets(backup, udid, card)?;
    let session = ActiveDeviceSession::open(Some(udid), ConnectionMode::Usb)?;
    let afc = AfcClient::new(&session)?;
    let refs: Vec<(&str, &[u8])> = originals
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_slice()))
        .collect();
    transfer_with_refresh(&session, &afc, card, &refs, log)
}
