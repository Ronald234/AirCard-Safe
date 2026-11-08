//! Research-only visual inventory. No device access or write authorization.
use serde::{Deserialize, Deserializer, Serialize, de};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Layout {
    Generic,
    StoreCard,
    Coupon,
    BoardingPass,
    EventTicket,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct VisualInventory {
    pub layout: Layout,
    pub nfc_declared: bool,
    pub images: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryError {
    InvalidMetadata,
    UnsupportedLayout,
    InvalidInventory,
    NoSupportedImages,
}

// Consume an object without retaining its keys or values.
struct Object;
impl<'de> Deserialize<'de> for Object {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Object;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an object")
            }
            fn visit_map<M: de::MapAccess<'de>>(self, mut map: M) -> Result<Object, M::Error> {
                while map.next_entry::<de::IgnoredAny, de::IgnoredAny>()?.is_some() {}
                Ok(Object)
            }
        }
        deserializer.deserialize_map(Visitor)
    }
}

struct Header {
    layout: Option<Layout>,
    nfc_declared: bool,
}
impl<'de> Deserialize<'de> for Header {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Header;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("pass metadata")
            }
            fn visit_map<M: de::MapAccess<'de>>(self, mut map: M) -> Result<Header, M::Error> {
                let mut seen = BTreeSet::new();
                let mut header = Header {
                    layout: None,
                    nfc_declared: false,
                };
                while let Some(key) = map.next_key::<String>()? {
                    if seen.len() >= 256 || !seen.insert(key.clone()) {
                        return Err(de::Error::custom("duplicate or excessive metadata"));
                    }
                    let layout = match key.as_str() {
                        "generic" => Some(Layout::Generic),
                        "storeCard" => Some(Layout::StoreCard),
                        "coupon" => Some(Layout::Coupon),
                        "boardingPass" => Some(Layout::BoardingPass),
                        "eventTicket" => Some(Layout::EventTicket),
                        _ => None,
                    };
                    if let Some(layout) = layout {
                        map.next_value::<Object>()?;
                        if header.layout.replace(layout).is_some() {
                            return Err(de::Error::custom("conflicting layouts"));
                        }
                    } else if key == "nfc" {
                        map.next_value::<Object>()?;
                        header.nfc_declared = true;
                    } else {
                        map.next_value::<de::IgnoredAny>()?;
                    }
                }
                Ok(header)
            }
        }
        deserializer.deserialize_map(Visitor)
    }
}

/// Inspect caller-supplied metadata and root filenames from the same pass.
/// NFC presence is not credential validation or a confirmed access technology.
/// The result is not a write permit and does not establish device provenance.
pub fn inspect(metadata: &[u8], names: &[&str]) -> Result<VisualInventory, InventoryError> {
    if metadata.len() > 256 * 1024 {
        return Err(InventoryError::InvalidMetadata);
    }
    let header: Header =
        serde_json::from_slice(metadata).map_err(|_| InventoryError::InvalidMetadata)?;
    let layout = header.layout.ok_or(InventoryError::UnsupportedLayout)?;
    if names.len() > 512 {
        return Err(InventoryError::InvalidInventory);
    }
    let mut seen = BTreeSet::new();
    for name in names {
        if name.is_empty()
            || name.len() > 128
            || matches!(*name, "." | "..")
            || name.chars().any(|c| c.is_control() || "/\\:".contains(c))
            || !seen.insert(name.to_ascii_lowercase())
        {
            return Err(InventoryError::InvalidInventory);
        }
    }
    let families: &[&str] = match layout {
        Layout::Generic => &["logo", "thumbnail"],
        Layout::StoreCard | Layout::Coupon => &["logo", "strip"],
        Layout::BoardingPass => &["logo", "footer"],
        Layout::EventTicket => &["logo", "strip", "background", "thumbnail"],
    };
    let mut images = Vec::new();
    for family in families {
        for scale in ["", "@2x", "@3x"] {
            let leaf = format!("{family}{scale}.png");
            if names.contains(&leaf.as_str()) {
                images.push(leaf);
            }
        }
    }
    images.sort();
    if images.is_empty() {
        return Err(InventoryError::NoSupportedImages);
    }
    Ok(VisualInventory {
        layout,
        nfc_declared: header.nfc_declared,
        images,
    })
}
