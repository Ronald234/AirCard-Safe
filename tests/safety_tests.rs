#[path = "../src/safety.rs"]
mod safety;

#[test]
fn original_cache_refresh_is_scoped_to_the_selected_cards_six_cache_leaves() {
    let card = "OM6NYhwXMZrAw0sRUjR62wmF4ZQ=";
    let items = [
        ("FrontFace", b"corrupted".as_slice()),
        ("PlaceHolder", b"corrupted".as_slice()),
        ("Preview", b"corrupted".as_slice()),
    ];
    assert_eq!(
        safety::wallet_cache_target(card, safety::WalletBatchKind::Cache, &items).unwrap(),
        format!("/var/mobile/Library/Passes/Cards/{card}.cache")
    );
    assert_eq!(
        safety::wallet_cache_target(card, safety::WalletBatchKind::PkCache, &items).unwrap(),
        format!("/var/mobile/Library/Passes/Cards/{card}.pkcache")
    );
    assert!(
        safety::wallet_cache_target("../escape", safety::WalletBatchKind::Cache, &items).is_err()
    );
    assert!(safety::wallet_cache_target(card, safety::WalletBatchKind::Artwork, &items).is_err());
    assert!(
        safety::wallet_cache_target(card, safety::WalletBatchKind::Cache, &items[..2]).is_err()
    );
    let mut invalid = items;
    invalid[0] = ("../pass.json", b"corrupted");
    assert!(safety::wallet_cache_target(card, safety::WalletBatchKind::Cache, &invalid).is_err());
    invalid[0] = ("FrontFace", b"arbitrary data");
    assert!(safety::wallet_cache_target(card, safety::WalletBatchKind::Cache, &invalid).is_err());
}

#[test]
fn successful_original_sequence_includes_both_refresh_batches_after_artwork() {
    use safety::WalletBatchKind::*;
    let mut submitted = Vec::new();
    safety::run_wallet_sequence(|kind| {
        submitted.push(kind);
        Ok(())
    })
    .unwrap();
    assert_eq!(submitted, vec![Artwork, Cache, PkCache]);
}

#[test]
fn failed_artwork_or_cache_stops_the_sequence_without_retrying() {
    use safety::WalletBatchKind::*;
    for fail_at in [Artwork, Cache, PkCache] {
        let mut submitted = Vec::new();
        let outcome = safety::run_wallet_sequence(|kind| {
            submitted.push(kind);
            if kind == fail_at {
                Err("transfer or cleanup failed".into())
            } else {
                Ok(())
            }
        });
        assert!(outcome.unwrap_err().contains("transfer or cleanup failed"));
        let expected = match fail_at {
            Artwork => vec![Artwork],
            Cache => vec![Artwork, Cache],
            PkCache => vec![Artwork, Cache, PkCache],
        };
        assert_eq!(submitted, expected);
    }
}

#[test]
fn original_wallet_batch_accepts_only_complete_artwork() {
    let png = b"\x89PNG\r\n\x1a\nexample";
    let pdf = b"%PDF-1.7 example";
    let card = "OM6NYhwXMZrAw0sRUjR62wmF4ZQ=";
    let items = [
        (safety::ARTWORK[0], png.as_slice()),
        (safety::ARTWORK[1], png.as_slice()),
        (safety::ARTWORK[2], pdf.as_slice()),
    ];
    assert!(safety::wallet_batch_target(card, &items).is_ok());
    assert!(safety::wallet_batch_target("../../escape", &items).is_err());
    assert!(safety::wallet_batch_target(card, &items[..2]).is_err());
    let mut wrong = items;
    wrong[2] = ("pass.json", pdf);
    assert!(safety::wallet_batch_target(card, &wrong).is_err());
    wrong[2] = (safety::ARTWORK[2], b"corrupted");
    assert!(safety::wallet_batch_target(card, &wrong).is_err());
    wrong[2] = (safety::ARTWORK[2], pdf);
    wrong[0] = (safety::ARTWORK[1], png);
    assert!(safety::wallet_batch_target(card, &wrong).is_err());
}

#[test]
fn staging_readback_rejects_missing_partial_or_changed_payloads() {
    let expected = [("stage/payload_0", b"original".as_slice())];
    assert!(safety::verify_staged_payloads(&expected, |_| Ok(b"original".to_vec())).is_ok());
    assert!(safety::verify_staged_payloads(&expected, |_| Ok(b"orig".to_vec())).is_err());
    assert!(safety::verify_staged_payloads(&expected, |_| Ok(b"modified".to_vec())).is_err());
    let error =
        safety::verify_staged_payloads(&expected, |_| Err("AFC disconnected".into())).unwrap_err();
    assert!(error.contains("AFC disconnected"));
}

#[test]
fn books_restoration_allows_empty_original_files() {
    use std::{cell::RefCell, collections::HashMap};
    let files = RefCell::new(HashMap::<String, Vec<u8>>::new());
    safety::restore_files(
        &[("Books/empty".into(), Some(vec![]))],
        |p| Ok(files.borrow().get(p).cloned()),
        |p, b| {
            files.borrow_mut().insert(p.into(), b.to_vec());
            Ok(())
        },
        |p| {
            files.borrow_mut().remove(p);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(files.borrow().get("Books/empty"), Some(&Vec::new()));
}

#[test]
fn books_presence_errors_fail_but_remaining_files_are_restored() {
    use std::{cell::RefCell, collections::HashMap};
    let files = RefCell::new(HashMap::<String, Vec<u8>>::new());
    let outcome = safety::restore_files(
        &[
            ("unreadable".into(), None),
            ("other".into(), Some(b"saved".to_vec())),
        ],
        |p| {
            if p == "unreadable" {
                Err("USB disconnected".into())
            } else {
                Ok(files.borrow().get(p).cloned())
            }
        },
        |p, b| {
            files.borrow_mut().insert(p.into(), b.to_vec());
            Ok(())
        },
        |_| Ok(()),
    );
    assert!(outcome.unwrap_err().contains("USB disconnected"));
    assert_eq!(files.borrow().get("other").unwrap(), b"saved");
}

#[test]
fn books_failed_removal_or_wrong_readback_is_not_success() {
    let snapshot = [("new".into(), None)];
    assert!(
        safety::restore_files(
            &snapshot,
            |_| Ok(Some(b"still here".to_vec())),
            |_, _| Ok(()),
            |_| Ok(())
        )
        .is_err()
    );
    let original = [("existing".into(), Some(b"original".to_vec()))];
    assert!(
        safety::restore_files(
            &original,
            |_| Ok(Some(b"changed".to_vec())),
            |_, _| Ok(()),
            |_| Ok(())
        )
        .is_err()
    );
}

#[test]
fn wallet_paths_reject_escape_and_keyboard_targets() {
    assert!(safety::valid_target(
        "/var/mobile/Library/Passes/Cards/OM6NYhwXMZrAw0sRUjR62wmF4ZQ=.pkpass",
        "cardBackgroundCombined@2x.png"
    ));
    for dir in [
        "/var/mobile/Library/Passes/Cards/../x.pkpass",
        "/var/mobile/Library/Passes/Cards/a/b.pkpass",
        "/var/mobile/Library/Caches/TelephonyUI-10",
        "/var/mobile/Library/Passes/Cards/a.pkpass/",
    ] {
        assert!(!safety::valid_target(dir, "cardBackgroundCombined@2x.png"));
    }
    assert!(!safety::valid_target(
        "/var/mobile/Library/Passes/Cards/OM6NYhwXMZrAw0sRUjR62wmF4ZQ=.pkpass",
        "../x"
    ));
}

#[test]
fn identifiers_reject_slashes_quotes_and_padding_in_middle() {
    for value in [
        "../escape",
        "aaaaaaaaaaaa/aaaaaaaaaaaaaa",
        "'OM6NYhwXMZrAw0sRUjR62wmF4ZQ='",
        "OM6NYhwXMZrAw0=RUjR62wmF4ZQ=",
    ] {
        assert!(!safety::valid_card_component(value));
    }
    assert!(safety::valid_card_component("OM6NYhwXMZrAw0sRUjR62wmF4ZQ="));
}

#[test]
fn altered_or_missing_backup_bytes_fail_verification() {
    assert!(safety::verify_bytes(b"original", b"original").is_ok());
    assert!(safety::verify_bytes(b"original", b"changed").is_err());
    assert!(safety::verify_bytes(b"", b"").is_err());
}

#[test]
fn usb_requires_usb_and_rejects_ambiguous_network_route() {
    assert!(safety::usb_route_allowed(true, false));
    assert!(!safety::usb_route_allowed(false, false));
    assert!(!safety::usb_route_allowed(false, true));
    assert!(!safety::usb_route_allowed(true, true));
}

#[test]
fn failed_write_still_restores_and_cleans_and_retains_all_errors() {
    use std::cell::RefCell;
    let phases = RefCell::new(Vec::new());
    let result = safety::run_with_cleanup(
        || {
            phases.borrow_mut().push("write");
            Err("write disconnected".into())
        },
        || {
            phases.borrow_mut().push("restore");
            Err("restore failed".into())
        },
        || {
            phases.borrow_mut().push("cleanup");
            Err("cleanup failed".into())
        },
    )
    .unwrap_err();
    assert_eq!(*phases.borrow(), vec!["write", "restore", "cleanup"]);
    assert!(result.contains("write disconnected"));
    assert!(result.contains("restore failed"));
    assert!(result.contains("cleanup failed"));
}

#[test]
fn cleanup_failure_is_never_reported_as_success() {
    assert!(
        safety::run_with_cleanup(|| Ok(()), || Ok(()), || Err("unlink failed".into())).is_err()
    );
    assert!(
        safety::run_with_cleanup(|| Ok(()), || Err("restore failed".into()), || Ok(())).is_err()
    );
    assert!(safety::run_with_cleanup(|| Ok(()), || Ok(()), || Ok(())).is_ok());
}

#[test]
fn first_cleanup_failure_stops_later_wallet_phases_and_keeps_all_recovery_attempts() {
    use std::cell::RefCell;
    let events = RefCell::new(Vec::new());
    let result = safety::run_wallet_sequence(|kind| {
        events.borrow_mut().push(format!("{kind:?}"));
        safety::run_with_cleanup(
            || {
                events.borrow_mut().push("work".into());
                Ok(())
            },
            || {
                events.borrow_mut().push("restore".into());
                Err("restore failed".into())
            },
            || {
                events.borrow_mut().push("cleanup".into());
                Err("cleanup failed".into())
            },
        )
    })
    .unwrap_err();
    assert_eq!(*events.borrow(), ["Artwork", "work", "restore", "cleanup"]);
    assert!(result.contains("restore failed"));
    assert!(result.contains("cleanup failed"));
}
