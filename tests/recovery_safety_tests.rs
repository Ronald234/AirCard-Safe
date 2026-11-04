#[path = "../src/crypto.rs"]
mod crypto;
#[path = "../src/flasher.rs"]
mod flasher;
#[path = "../src/safety.rs"]
mod safety;
#[path = "../src/wallet_backup.rs"]
mod wallet_backup;

#[test]
fn locally_hashed_complete_backup_is_not_verified_original_or_recovery() {
    let dir = std::env::temp_dir().join(format!("aircard-recovery-{}", crypto::token().unwrap()));
    std::fs::create_dir(&dir).unwrap();
    let device = "synthetic-device";
    let card = "AAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    let data: [&[u8]; 3] = [
        b"\x89PNG\r\n\x1a\nlocal-a",
        b"\x89PNG\r\n\x1a\nlocal-b",
        b"%PDF-local",
    ];
    for (leaf, bytes) in safety::ARTWORK.iter().zip(data) {
        std::fs::write(dir.join(leaf), bytes).unwrap();
    }
    let manifest = serde_json::json!({
        "version": 1,
        "binding": crypto::sha256(format!("AirCard-Safe-v1\0{device}\0{}", card.trim_end_matches('=')).as_bytes()).unwrap(),
        "digests": data.map(|b| crypto::sha256(b).unwrap())
    });
    std::fs::write(
        dir.join("backup.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let manifest_bytes = std::fs::read(dir.join("backup.json")).unwrap();
    assert_eq!(
        wallet_backup::inspect_unverified_local_assets(&dir, device, card)
            .unwrap()
            .len(),
        3
    );
    assert!(
        wallet_backup::inspect_unverified_local_assets(&dir, "other-synthetic-device", card)
            .is_err()
    );
    assert!(
        wallet_backup::inspect_unverified_local_assets(
            &dir,
            device,
            "BBBBBBBBBBBBBBBBBBBBBBBBBBB="
        )
        .is_err()
    );
    for (test_device, test_card) in [
        (device, card),
        ("other-synthetic-device", card),
        (device, "BBBBBBBBBBBBBBBBBBBBBBBBBBB="),
    ] {
        assert!(wallet_backup::load_original_assets(&dir, test_device, test_card).is_err());
        assert_eq!(
            flasher::restore(test_device, test_card, &dir, |_| panic!(
                "blocked restore must not log transfer progress"
            ))
            .unwrap_err()
            .to_string(),
            safety::RECOVERY_BLOCK_REASON
        );
    }
    assert_eq!(
        std::fs::read(dir.join("backup.json")).unwrap(),
        manifest_bytes
    );
    for (leaf, expected) in safety::ARTWORK.iter().zip(data) {
        assert_eq!(std::fs::read(dir.join(leaf)).unwrap(), expected);
    }
    // Corruption, missing originals and unsupported/foreign formats never authorize restore.
    std::fs::write(dir.join(safety::ARTWORK[0]), b"tampered").unwrap();
    assert!(wallet_backup::inspect_unverified_local_assets(&dir, device, card).is_err());
    assert!(wallet_backup::load_original_assets(&dir, device, card).is_err());
    std::fs::write(dir.join(safety::ARTWORK[0]), data[0]).unwrap();
    std::fs::remove_file(dir.join(safety::ARTWORK[1])).unwrap();
    assert!(wallet_backup::inspect_unverified_local_assets(&dir, device, card).is_err());
    assert!(wallet_backup::load_original_assets(&dir, device, card).is_err());
    std::fs::write(dir.join(safety::ARTWORK[1]), data[1]).unwrap();
    let mut foreign = manifest.clone();
    foreign["version"] = 99.into();
    std::fs::write(
        dir.join("backup.json"),
        serde_json::to_vec(&foreign).unwrap(),
    )
    .unwrap();
    assert!(wallet_backup::inspect_unverified_local_assets(&dir, device, card).is_err());
    let result = wallet_backup::load_original_assets(&dir, device, card);
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(
        result.is_err(),
        "A complete self-authored manifest is not evidence of protected-file origin or validated recovery"
    );
}

#[test]
fn valid_apply_and_restore_are_blocked_without_device_modules_or_log_side_effects() {
    let card = "AAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    let png = b"\x89PNG\r\n\x1a\nsynthetic";
    let pdf = b"%PDF-synthetic";
    let mut logged = Vec::new();
    let error = flasher::apply("synthetic-device", card, png, png, pdf, |m| {
        logged.push(m.to_string())
    })
    .unwrap_err();
    assert_eq!(error.to_string(), safety::RECOVERY_BLOCK_REASON);
    let error = flasher::restore(
        "synthetic-device",
        card,
        std::path::Path::new("missing-backup"),
        |m| logged.push(m.to_string()),
    )
    .unwrap_err();
    assert_eq!(error.to_string(), safety::RECOVERY_BLOCK_REASON);
    assert!(logged.is_empty());
    // This integration crate does not provide device, AFC, AirTraffic or staging
    // modules: the real entry points cannot compile if they regain those effects.
}
