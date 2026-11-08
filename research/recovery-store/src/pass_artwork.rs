//! Research-only visual inventory. No device access or write authorization.
use serde::Serialize;

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

/// Metadata and file names must come from the same supplied pass.
/// Results describe candidates only, never provenance, rendering or recovery.
pub fn inspect(
    _metadata: &[u8],
    _names: &[&str],
) -> Result<VisualInventory, InventoryError> {
    Err(InventoryError::UnsupportedLayout)
}
