#[path = "../src/safety.rs"]
mod safety;

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
