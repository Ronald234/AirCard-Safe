use aircard_recovery_store_lab::{capture, inspect};
use std::fs;
const BINDING: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
fn originals() -> [Option<&'static [u8]>; 3] {
    [
        Some(b"exact-original-png-3x"),
        None,
        Some(b"exact-original-pdf"),
    ]
}
#[test]
fn restart_preserves_exact_bytes_and_explicit_absence() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("capture");
    let saved = capture(&root, BINDING, originals()).unwrap();
    assert_eq!(saved.asset(0), originals()[0]);
    drop(saved);
    let reopened = inspect(&root, BINDING).unwrap();
    for (index, expected) in originals().into_iter().enumerate() {
        assert_eq!(reopened.asset(index), expected);
    }
}
#[test]
fn an_existing_directory_and_its_files_are_never_overwritten() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("capture");
    capture(&root, BINDING, originals()).unwrap();
    let before = fs::read(root.join("original-0.bin")).unwrap();
    assert!(capture(&root, BINDING, [Some(b"replace"), None, None]).is_err());
    assert_eq!(fs::read(root.join("original-0.bin")).unwrap(), before);
}
#[test]
fn incomplete_or_corrupt_capture_is_rejected_without_deleting_the_surviving_copy() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("capture");
    capture(&root, BINDING, originals()).unwrap();
    fs::remove_file(root.join("manifest.json")).unwrap();
    assert!(inspect(&root, BINDING).is_err());
    assert_eq!(
        fs::read(root.join("original-0.bin")).unwrap(),
        originals()[0].unwrap()
    );
}
#[test]
fn wrong_binding_and_changed_original_are_rejected() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("capture");
    capture(&root, BINDING, originals()).unwrap();
    assert!(inspect(&root, &"b".repeat(64)).is_err());
    fs::write(root.join("original-0.bin"), b"tampered").unwrap();
    assert!(inspect(&root, BINDING).is_err());
}
#[test]
fn unknown_manifest_fields_cannot_assert_verified_recovery() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("capture");
    capture(&root, BINDING, originals()).unwrap();
    let path = root.join("manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    manifest["recovery_verified"] = true.into();
    fs::write(path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(inspect(&root, BINDING).is_err());
}
#[test]
fn unexpected_files_in_an_absent_slot_are_a_conflict() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("capture");
    capture(&root, BINDING, originals()).unwrap();
    fs::write(root.join("original-1.bin"), b"untracked").unwrap();
    assert!(inspect(&root, BINDING).is_err());
}
#[test]
fn empty_capture_and_invalid_binding_create_nothing() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("capture");
    assert!(capture(&root, BINDING, [None, None, None]).is_err());
    assert!(!root.exists());
    assert!(capture(&root, "raw-device-id", originals()).is_err());
    assert!(!root.exists());
}
#[cfg(unix)]
#[test]
fn a_symlink_cannot_replace_an_original() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("capture");
    capture(&root, BINDING, originals()).unwrap();
    let outside = parent.path().join("outside");
    fs::write(&outside, originals()[0].unwrap()).unwrap();
    fs::remove_file(root.join("original-0.bin")).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("original-0.bin")).unwrap();
    assert!(inspect(&root, BINDING).is_err());
    assert_eq!(fs::read(outside).unwrap(), originals()[0].unwrap());
}

#[cfg(unix)]
#[test]
fn a_fifo_manifest_is_rejected_without_waiting_for_a_writer() {
    use std::{ffi::CString, sync::mpsc, time::Duration};
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("capture");
    fs::create_dir(&root).unwrap();
    let path = CString::new(root.join("manifest.json").as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(inspect(&root, BINDING).is_err());
    });
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(1)),
        Ok(true),
        "inspection must not block opening a FIFO"
    );
}
