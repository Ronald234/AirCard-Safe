use aircard_recovery_store_lab::pass_artwork::{InventoryError, Layout, inspect};

#[test]
fn nfc_store_card_uses_existing_visual_assets_without_exporting_credentials() {
    let metadata = br#"{"storeCard":{},"nfc":{"message":"SECRET_CREDENTIAL","encryptionPublicKey":"SECRET_KEY"},"authenticationToken":"SECRET_TOKEN","serialNumber":"SECRET_SERIAL"}"#;
    let inventory = inspect(
        metadata,
        &[
            "pass.json",
            "manifest.json",
            "signature",
            "strip@2x.png",
            "strip@3x.png",
            "thumbnail.png",
            "cardBackgroundCombined@3x.png",
        ],
    )
    .unwrap();
    assert_eq!(inventory.layout, Layout::StoreCard);
    assert!(inventory.nfc_declared);
    assert_eq!(inventory.images, vec!["strip@2x.png", "strip@3x.png"]);
    let output = serde_json::to_string(&inventory).unwrap();
    for forbidden in ["SECRET", "pass.json", "signature", "manifest.json"] {
        assert!(!output.contains(forbidden));
    }
}

#[test]
fn each_layout_selects_only_its_existing_image_families() {
    for (metadata, expected) in [
        (br#"{"generic":{}}"#.as_slice(), vec!["logo.png", "thumbnail.png"]),
        (br#"{"storeCard":{}}"#.as_slice(), vec!["logo.png", "strip.png"]),
        (br#"{"coupon":{}}"#.as_slice(), vec!["logo.png", "strip.png"]),
        (br#"{"boardingPass":{}}"#.as_slice(), vec!["footer.png", "logo.png"]),
        (
            br#"{"eventTicket":{}}"#.as_slice(),
            vec!["background.png", "logo.png", "strip.png", "thumbnail.png"],
        ),
    ] {
        let inventory = inspect(
            metadata,
            &["strip.png", "logo.png", "thumbnail.png", "footer.png", "background.png"],
        )
        .unwrap();
        assert_eq!(inventory.images, expected);
        assert!(!inventory.nfc_declared);
    }
}

#[test]
fn invalid_ambiguous_or_oversized_metadata_never_creates_candidates() {
    for metadata in [
        br#"{"generic":{},"storeCard":{}}"#.as_slice(),
        br#"{"generic":{},"generic":{}}"#.as_slice(),
        br#"{"generic":[]}"#.as_slice(),
        br#"{"generic":{},"nfc":"SECRET"}"#.as_slice(),
        br#"{"generic":{},"nfc":{},"nfc":{}}"#.as_slice(),
        br#"{"generic":{}} trailing"#.as_slice(),
    ] {
        assert_eq!(
            inspect(metadata, &["logo.png"]),
            Err(InventoryError::InvalidMetadata)
        );
    }
    assert_eq!(
        inspect(&vec![b' '; 262_145], &["logo.png"]),
        Err(InventoryError::InvalidMetadata)
    );
    assert_eq!(
        inspect(br#"{"paymentCard":{}}"#, &["logo.png"]),
        Err(InventoryError::UnsupportedLayout)
    );
}

#[test]
fn paths_aliases_and_missing_images_are_not_silently_guessed() {
    for names in [
        vec!["../strip.png"],
        vec!["folder/strip.png"],
        vec!["C:strip.png"],
        vec!["strip.png", "STRIP.PNG"],
        vec!["strip.png", "strip.png"],
        vec!["strip.png\0"],
    ] {
        assert_eq!(
            inspect(br#"{"storeCard":{}}"#, &names),
            Err(InventoryError::InvalidInventory)
        );
    }
    assert_eq!(
        inspect(br#"{"storeCard":{}}"#, &["pass.json", "signature"]),
        Err(InventoryError::NoSupportedImages)
    );
    assert_eq!(
        inspect(br#"{"generic":{}}"#, &["strip.png"]),
        Err(InventoryError::NoSupportedImages)
    );
    assert_eq!(
        inspect(br#"{"generic":{}}"#, &vec!["logo.png"; 513]),
        Err(InventoryError::InvalidInventory)
    );
}
