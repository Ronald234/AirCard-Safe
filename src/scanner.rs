use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use anyhow::Result;
use regex::Regex;

use crate::device::ConnectionMode;

#[path = "scanner/runtime.rs"]
mod runtime;
#[path = "scanner/unified.rs"]
mod unified;

pub use runtime::{scan_wallet_items, scan_wallet_items_with_diagnostics};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassKind {
    Generic,
    Loyalty,
    Transit,
    Access,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WalletItem {
    PaymentCard(String),
    UnsupportedPass(String),
    CandidatePass(String),
    ClassifiedPass(String, PassKind),
}

fn explicit_kind(line: &str) -> (Option<PassKind>, bool) {
    static KIND: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
        r#"(?i)(?:^|[\s{,;])(?:"?(?:passType|passKind)"?\s*[:=]\s*"?(generic|loyalty|transit|access)"?(?:[\s,;}\"]|$)|"(generic|storeCard|boardingPass)"\s*:\s*\{)"#
    ).unwrap()
    });
    let mut kind = None;
    for capture in KIND.captures_iter(line) {
        let value = capture
            .get(1)
            .or_else(|| capture.get(2))
            .unwrap()
            .as_str()
            .to_ascii_lowercase();
        let next = match value.as_str() {
            "generic" => PassKind::Generic,
            "loyalty" | "storecard" => PassKind::Loyalty,
            "transit" => PassKind::Transit,
            "access" => PassKind::Access,
            // A boarding pass is a generic nonpayment layout, not a transit credential.
            "boardingpass" => PassKind::Generic,
            _ => continue,
        };
        if kind.is_some_and(|previous| previous != next) {
            return (None, true);
        }
        kind = Some(next);
    }
    (kind, false)
}

fn has_wallet_context(line: &str) -> bool {
    static CONTEXT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
        // Source context belongs to the prefix, never a quoted/embedded
        // process or framework name in an unrelated logger's message.
        r#"(?i)^\s*(?:(?:(?:[a-z]{3}\s+[0-9]{1,2}\s+|[0-9]{4}-[0-9]{2}-[0-9]{2}[T ])[0-9]{2}:[0-9]{2}:[0-9]{2}(?:\.[0-9]+)?(?:Z|[+-][0-9]{2}:?[0-9]{2})?\s+\S+\s+)?(?:passd|nfcd|nanopassd|passbook|npkcompanion|passkit|passkitui)(?:[\s:(\[]|$)|[^:\r\n()]{1,128}\((?:passkit|passkitui|stockholm|wallet)\):|(?:PDCardFileManager|PDPassLibrary|VerificationCheck)[:.])"#
    ).unwrap()
    });
    line.len() <= 128 * 1024 && CONTEXT.is_match(line)
}

fn extract_wallet_items(line: &str) -> Vec<WalletItem> {
    static PATHS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
        r#"(?i)/(?:(?:var/mobile/Library/Passes/)?Cards)/([-A-Za-z0-9_+=]{20,128})(?:\.(?:pkpass|cache|pkcache))?(?:[/\s"'),]|$)"#
    ).unwrap()
    });
    static SELECTED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
        r#"(?i)(?:selected pass uniqueID\s*:\s*|PDCardFileManager:\s*writing card\s+|PDPassLibrary:\s*wrote pass\s+|VerificationCheck\.)(?:"([-A-Za-z0-9_.+=]{1,128})"|'([-A-Za-z0-9_.+=]{1,128})'|([-A-Za-z0-9_.+=]{1,128})(?:[\s),]|$))"#
    ).unwrap()
    });
    static IN_SESSION: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?i)passIDs\[InSession\]\s*:\s*(?:\{\s*)?\(([^)]{0,8192})\)"#).unwrap()
    });
    static LIST_ID: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#""([-A-Za-z0-9_+=]{20,128})""#).unwrap());
    static MEMBERSHIP_CONTEXT: LazyLock<Regex> = LazyLock::new(|| {
        // Field/layout evidence must be outside an opaque identifier. An ID
        // containing "membership" is not itself evidence of a pass layout.
        Regex::new(r#"(?i)(?:^|[\s:("'])(?:membership|storecard|generic pass|boardingpass|eventticket|coupon|apple card|applecard)(?:[\s:),"']|$)|"generic"\s*:|/(?:thumbnail|strip|artwork)(?:@[23]x)?\.png(?:[\s"'),]|$)"#).unwrap()
    });
    static DASHBOARD: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
        r#"(?i)Dashboard loading[^:\r\n]{0,512}:\s*(?:for\s+([-A-Za-z0-9_+=]{20,128})(?:[,\s"')]|$)|([-A-Za-z0-9_+=]{20,128})\s+-)"#
    ).unwrap()
    });
    if line.len() > 128 * 1024 {
        return Vec::new();
    }
    if !has_wallet_context(line) {
        return Vec::new();
    }
    let (kind, conflicting_kind) = explicit_kind(line);
    let membership = conflicting_kind || MEMBERSHIP_CONTEXT.is_match(line);
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for capture in PATHS.captures_iter(line) {
        if seen.len() >= 256 {
            break;
        }
        let id = &capture[1];
        if seen.insert(id.to_string()) {
            result.push(if let Some(kind) = kind {
                WalletItem::ClassifiedPass(id.into(), kind)
            } else if membership {
                WalletItem::UnsupportedPass(id.into())
            } else if is_valid_card_hash(id) {
                WalletItem::PaymentCard(id.into())
            } else {
                WalletItem::CandidatePass(id.into())
            });
        }
    }
    for capture in SELECTED.captures_iter(line) {
        if seen.len() >= 256 {
            break;
        }
        let id = capture
            .get(1)
            .or_else(|| capture.get(2))
            .or_else(|| capture.get(3))
            .unwrap()
            .as_str();
        if [
            ".", "..", "...", "null", "nil", "none", "unknown", "private", "redacted",
        ]
        .iter()
        .any(|placeholder| id.eq_ignore_ascii_case(placeholder))
        {
            continue;
        }
        if seen.insert(id.to_string()) {
            result.push(if let Some(kind) = kind {
                WalletItem::ClassifiedPass(id.into(), kind)
            } else if membership {
                WalletItem::UnsupportedPass(id.into())
            } else {
                WalletItem::CandidatePass(id.into())
            });
        }
    }
    // Express-mode global configuration is stale-capable and is never inventory.
    for list in IN_SESSION.captures_iter(line) {
        for capture in LIST_ID.captures_iter(&list[1]) {
            if seen.len() >= 256 {
                break;
            }
            let id = &capture[1];
            if seen.insert(id.to_string()) {
                result.push(if let Some(kind) = kind {
                    WalletItem::ClassifiedPass(id.into(), kind)
                } else if membership {
                    WalletItem::UnsupportedPass(id.into())
                } else {
                    WalletItem::CandidatePass(id.into())
                });
            }
        }
    }
    for capture in DASHBOARD.captures_iter(line) {
        if seen.len() >= 256 {
            break;
        }
        let id = capture.get(1).or_else(|| capture.get(2)).unwrap().as_str();
        if seen.insert(id.to_string()) {
            result.push(if let Some(kind) = kind {
                WalletItem::ClassifiedPass(id.into(), kind)
            } else if membership {
                WalletItem::UnsupportedPass(id.into())
            } else {
                WalletItem::CandidatePass(id.into())
            });
        }
    }
    result
}

#[cfg(windows)]
unsafe extern "system" {
    fn setsockopt(s: usize, level: i32, optname: i32, optval: *const i8, optlen: i32) -> i32;
}

#[cfg(windows)]
const SOL_SOCKET: i32 = 0xffff;
#[cfg(windows)]
const SO_RCVTIMEO: i32 = 0x1006;

pub fn is_valid_card_hash(h: &str) -> bool {
    if !crate::safety::valid_card_component(h) {
        return false;
    }
    let trimmed = h.trim_matches(['\'', '"']).trim_end_matches(['.', ',']);
    let len = trimmed.len();
    // Real Apple Wallet card hashes are SHA-1 (27-28 chars) or SHA-256 (43-44 chars)
    if len != 27 && len != 28 && len != 43 && len != 44 {
        return false;
    }

    // Must be base64 alphabet characters
    if !trimmed.chars().all(|c| {
        c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '-' || c == '_' || c == '='
    }) {
        return false;
    }

    // Reject obvious system identifiers, bundle IDs and common keywords
    let lower = trimmed.to_lowercase();
    if lower.contains("mobileasset")
        || lower.contains("com_apple")
        || lower.contains("com.")
        || lower.contains("apple.")
        || lower.contains("curtain")
        || lower.contains("binder")
        || lower.contains("optimizer")
        || lower.contains("system")
        || lower.contains("uaf")
        || lower.contains("siri")
        || lower.contains("dialog")
        || lower.contains("planner")
        || lower.contains("linguistic")
        || lower.contains("timing")
        || lower.contains("model")
        || lower.contains("translation")
        || lower.contains("visual")
        || lower.contains("device")
        || lower.contains("override")
        || lower.contains("motion")
        || lower.contains("search")
    {
        return false;
    }

    // '=' can only appear at the end
    if let Some(pos) = trimmed.find('=') {
        if pos < len - 2 {
            return false;
        }
    }

    // Normalize URL-safe base64 and pad
    let mut b64 = trimmed.replace('-', "+").replace('_', "/");
    while b64.len() % 4 != 0 {
        b64.push('=');
    }

    use base64::Engine;
    if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(&b64) {
        // Must be exactly 20 bytes (SHA-1) or 32 bytes (SHA-256)
        if decoded.len() == 20 || decoded.len() == 32 {
            // Reject trivial all-identical bytes
            if decoded.iter().all(|&b| b == decoded[0]) {
                return false;
            }

            // Cryptographic hashes have high byte entropy:
            // 1. Must contain both bytes with MSB set (>= 128) and MSB clear (< 128).
            let has_high = decoded.iter().any(|&b| b >= 128);
            let has_low = decoded.iter().any(|&b| b < 128);
            if !has_high || !has_low {
                return false;
            }

            // 2. Must contain at least 12 distinct byte values in 20 bytes
            let mut unique_bytes = std::collections::HashSet::new();
            for &b in &decoded {
                unique_bytes.insert(b);
            }
            if unique_bytes.len() < 12 {
                return false;
            }

            if DUMMY_HASHES.contains(&trimmed)
                || DUMMY_HASHES
                    .iter()
                    .any(|d| d.trim_end_matches('=') == trimmed)
            {
                return false;
            }
            return true;
        }
    }

    false
}

const WALLET_KEYWORDS: &[&str] = &[
    "pdcardfilemanager",
    "pdpasslibrary",
    "verificationcheck",
    "nfcd",
    "passd",
    "passbook",
    "passkit",
    "stockholm",
    "nanopassd",
    "npkcompanion",
    "wallet",
    "/cards/",
    "/passes/",
];

const DUMMY_HASHES: &[&str] = &[
    "M6nDwZrkYbFlsodLgCbvyFZQ1cc=",
    "kJL-D0rr-SZhbj2c8nK-OQ9hCMY=",
    "hwAtAmHKYwsQrJbT5cTNDsaxVME=",
];

use std::sync::LazyLock;

#[cfg(test)]
static DESC_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)(?:description|localizedDescription|passName|title)\s*[:=]\s*['"]([^'"]+)['"]"#,
    )
    .unwrap()
});

#[cfg(test)]
static CARD_REGEXES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"/(?:Cards|Passes/Cards)/([A-Za-z0-9+=_-]{27,44})(?:\.pkpass|\.cache|\.pkcache|/|\s|\x22|'|\)|,|$)").unwrap(),
        Regex::new(r"/([A-Za-z0-9+=_-]{27,44})\.(?:pkpass|cache|pkcache)").unwrap(),
        Regex::new(r"(?:^|[^A-Za-z0-9+/_-])([A-Za-z0-9+/_-]{27}=)(?:$|[^A-Za-z0-9+/_-])").unwrap(),
        Regex::new(r"(?i)(?:card[_\s]?(?:hash|id)|pass[_\s]?(?:hash|id)|unique[_\s]?id)\s*[:=]\s*['\x22]?([A-Za-z0-9+=_-]{27,44})").unwrap(),
    ]
});

#[cfg(test)]
pub fn extract_card_name_from_line(line: &str) -> Option<String> {
    if let Some(caps) = DESC_RE.captures(line) {
        if let Some(m) = caps.get(1) {
            let name = m.as_str().trim();
            if name.len() > 1 && !name.to_lowercase().contains("<private>") {
                return Some(name.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
pub fn extract_card_hash_from_line(line: &str) -> Option<String> {
    let lower = line.to_lowercase();
    let has_wallet = WALLET_KEYWORDS.iter().any(|k| lower.contains(k));
    if !has_wallet {
        return None;
    }

    for r in CARD_REGEXES.iter() {
        if let Some(caps) = r.captures(line) {
            if let Some(m) = caps.get(1) {
                let h = m
                    .as_str()
                    .trim()
                    .trim_matches(['\'', '"'])
                    .trim_end_matches(['.', ',']);
                if is_valid_card_hash(h) {
                    // Base64 decoding may need padding; the device path must keep
                    // the identifier exactly as observed, including its padding.
                    return Some(h.to_string());
                }
            }
        }
    }

    None
}

#[allow(dead_code)]
pub fn scan_syslog_for_cards<F, L>(
    udid: Option<&str>,
    connection_mode: ConnectionMode,
    stop_flag: Arc<AtomicBool>,
    mut on_card_found: F,
    log: L,
) -> Result<()>
where
    F: FnMut(String, String),
    L: FnMut(String),
{
    scan_wallet_items(
        udid,
        connection_mode,
        stop_flag,
        |item| {
            if let WalletItem::PaymentCard(id) = item {
                on_card_found(id, String::new());
            }
        },
        log,
    )
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallet_discovery_returns_all_concrete_card_paths() {
        let line = "passd: /var/mobile/Library/Passes/Cards/4FD0Y4djT-oIj--0Odq8GMLVpVo.pkpass/a.png /var/mobile/Library/Passes/Cards/99YLTeI09ke_k64JqGdQ_F4Ho7k.cache/FrontFace";
        assert_eq!(
            extract_wallet_items(line),
            vec![
                WalletItem::PaymentCard("4FD0Y4djT-oIj--0Odq8GMLVpVo".into()),
                WalletItem::PaymentCard("99YLTeI09ke_k64JqGdQ_F4Ho7k".into()),
            ]
        );
    }

    #[test]
    fn selected_pass_is_candidate_without_explicit_membership_evidence() {
        assert_eq!(
            extract_wallet_items("nfcd: selected pass uniqueID : \"4FD0Y4djT-oIj--0Odq8GMLVpVo\""),
            vec![WalletItem::CandidatePass(
                "4FD0Y4djT-oIj--0Odq8GMLVpVo".into()
            )]
        );
        assert_eq!(
            extract_wallet_items("passd: selected pass uniqueID : \"gym-membership-123456789\""),
            vec![WalletItem::CandidatePass("gym-membership-123456789".into())]
        );
    }

    #[test]
    fn structured_pass_events_are_read_only_without_a_process_prefix() {
        for line in [
            "PDCardFileManager: writing card gym-membership-123456789",
            "PDPassLibrary: wrote pass gym-membership-123456789",
            "VerificationCheck.gym-membership-123456789",
        ] {
            assert_eq!(
                extract_wallet_items(line),
                vec![WalletItem::CandidatePass("gym-membership-123456789".into())]
            );
        }
    }

    #[test]
    fn express_global_configuration_and_bare_tokens_are_not_inventory() {
        assert!(
            extract_wallet_items("nfcd: passIDs[global]: {(\n\"4FD0Y4djT-oIj--0Odq8GMLVpVo\"\n)}")
                .is_empty()
        );
        assert!(extract_wallet_items("passd: unrelated 4FD0Y4djT-oIj--0Odq8GMLVpVo").is_empty());
        assert!(extract_wallet_items("nfcd: selected pass uniqueID : <private>").is_empty());
        assert!(
            extract_wallet_items("nfcd: selected pass uniqueID : \"../../private-pass-123456789\"")
                .is_empty()
        );
    }

    #[test]
    fn membership_paths_never_reach_payment_writer_classification() {
        assert_eq!(
            extract_wallet_items(
                "passd: /var/mobile/Library/Passes/Cards/gym-membership-123456789.pkpass/thumbnail.png"
            ),
            vec![WalletItem::UnsupportedPass(
                "gym-membership-123456789".into()
            )]
        );
        assert_eq!(
            extract_wallet_items(
                "passd: membership /var/mobile/Library/Passes/Cards/4FD0Y4djT-oIj--0Odq8GMLVpVo.pkpass/thumbnail.png"
            ),
            vec![WalletItem::UnsupportedPass(
                "4FD0Y4djT-oIj--0Odq8GMLVpVo".into()
            )]
        );
    }

    #[test]
    fn explicit_apple_card_rendering_context_stays_read_only() {
        for kind in ["Apple Card", "AppleCard"] {
            let line = format!(
                "passd: {kind}: /var/mobile/Library/Passes/Cards/OM6NYhwXMZrAw0sRUjR62wmF4ZQ=.pkpass/cardBackgroundCombined@3x.png"
            );
            assert_eq!(
                extract_wallet_items(&line),
                vec![WalletItem::UnsupportedPass(
                    "OM6NYhwXMZrAw0sRUjR62wmF4ZQ=".into()
                )]
            );
        }
    }

    #[test]
    fn same_hash_keeps_explicit_membership_distinct_from_untyped_selection() {
        let id = "4FD0Y4djT-oIj--0Odq8GMLVpVo";
        assert_eq!(
            extract_wallet_items(&format!(
                "nfcd: membership selected pass uniqueID : \"{id}\""
            )),
            vec![WalletItem::UnsupportedPass(id.into())]
        );
        assert_eq!(
            extract_wallet_items(&format!("nfcd: selected pass uniqueID : \"{id}\"")),
            vec![WalletItem::CandidatePass(id.into())]
        );
        assert_eq!(
            extract_wallet_items(&format!("nfcd: passIDs[InSession]: {{(\"{id}\")}}")),
            vec![WalletItem::CandidatePass(id.into())]
        );
        assert_eq!(
            extract_wallet_items(&format!(
                "passd: /var/mobile/Library/Passes/Cards/{id}.pkpass"
            )),
            vec![WalletItem::PaymentCard(id.into())]
        );
    }

    #[test]
    fn test_extract_card_hash() {
        let line1 = "passd[123]: Card hash: 'OM6NYhwXMZrAw0sRUjR62wmF4ZQ=' loaded";
        assert_eq!(
            extract_card_hash_from_line(line1),
            Some("OM6NYhwXMZrAw0sRUjR62wmF4ZQ=".to_string())
        );

        let line2 = "nanopassd: Accessing /var/mobile/Library/Passes/Cards/d64fKk0kyHWP11IWV2GRLud4XQk.pkpass";
        assert_eq!(
            extract_card_hash_from_line(line2),
            Some("d64fKk0kyHWP11IWV2GRLud4XQk".to_string())
        );

        // Dummy/unrelated lines should be ignored
        let dummy = "passd: Using dummy hash hwAtAmHKYwsQrJbT5cTNDsaxVME=";
        assert_eq!(extract_card_hash_from_line(dummy), None);
    }

    #[test]
    fn test_extract_card_name() {
        let line = "passd[456]: Pass with localizedDescription = 'Apple Card' updated";
        assert_eq!(
            extract_card_name_from_line(line),
            Some("Apple Card".to_string())
        );
    }

    #[test]
    fn explicit_wallet_paths_preserve_padding_and_url_safe_characters() {
        for hash in [
            "OM6NYhwXMZrAw0sRUjR62wmF4ZQ=",
            "d64fKk0kyHWP11IWV2GRLud4XQk",
            "4FD0Y4djT-oIj--0Odq8GMLVpVo",
            "99YLTeI09ke_k64JqGdQ_F4Ho7k",
        ] {
            let line = format!("passd: /var/mobile/Library/Passes/Cards/{hash}.pkpass");
            assert_eq!(extract_card_hash_from_line(&line).as_deref(), Some(hash));
        }
    }

    #[test]
    fn test_is_valid_card_hash() {
        // Real card hashes
        assert!(is_valid_card_hash("OM6NYhwXMZrAw0sRUjR62wmF4ZQ="));
        assert!(is_valid_card_hash("d64fKk0kyHWP11IWV2GRLud4XQk="));
        assert!(is_valid_card_hash("d64fKk0kyHWP11IWV2GRLud4XQk"));

        // System garbage strings that must be rejected
        let garbage = [
            "PresentationBinderIndirectAccessHosting-",
            "SB-systemApertureCurtain",
            "com_apple_MobileAsset_UAF_Translation_Assets",
            "com_apple_MobileAsset_UAF_FM_Visual",
            "com_apple_MobileAsset_UAF_DeviceCheck",
            "com_apple_MobileAsset_UAF_Siri_TextToSpeech",
            "com_apple_MobileAsset_UAF_Siri_DialogAssets",
            "com_apple_MobileAsset_UAF_IF_Planner",
            "SubscriptionOptimizerTimingModels",
            "com_apple_MobileAsset_UAF_LinguisticData",
            "com_apple_MobileAsset_UAF_FM_Overrides",
            "com_apple_MobileAsset_UAF_Siri_Understanding",
            "com_apple_MobileAsset_UAF_MotionAnomalyFM",
            "com_apple_MobileAsset_UAF_TKModelMessages",
            "com_apple_MobileAsset_UAF_Search_ODLA",
        ];

        for g in garbage {
            assert!(
                !is_valid_card_hash(g),
                "Expected {} to be rejected as card hash",
                g
            );
        }
    }
}

#[cfg(test)]
mod revision4_tests {
    use super::*;
    #[test]
    fn dashboard_candidates_require_live_wallet_context_and_complete_identifier() {
        let id = "synthetic-pass-123456789";
        for message in [
            format!("Passbook(PassKitUI): Dashboard loading (...): for {id}, pass feature unknown"),
            format!("Passbook(PassKitUI): Dashboard loading (...): {id} - m:NO"),
        ] {
            assert_eq!(
                extract_wallet_items(&message),
                vec![WalletItem::CandidatePass(id.into())]
            );
        }
        assert!(
            extract_wallet_items(
                "webserver: wallet Dashboard loading (...): for synthetic-pass-123456789, password"
            )
            .is_empty()
        );
        assert!(
            extract_wallet_items(
                "Passbook: Dashboard loading (...): for ../../synthetic-pass-123456789, other"
            )
            .is_empty()
        );
    }
}

#[cfg(test)]
mod classification_v4_tests {
    use super::*;
    #[test]
    fn explicit_pass_layout_classification_is_not_inferred_from_identifier_or_issuer() {
        let id = "synthetic-pass-123456789";
        for (metadata, kind) in [
            ("passType: generic", "Generic"),
            ("passType: loyalty", "Loyalty"),
            ("passType: transit", "Transit"),
            ("passType: access", "Access"),
        ] {
            let line = format!("passd: {metadata}; selected pass uniqueID : \"{id}\"");
            assert_eq!(
                format!("{:?}", extract_wallet_items(&line)),
                format!("[ClassifiedPass(\"{id}\", {kind})]")
            );
        }
        assert_eq!(
            extract_wallet_items(
                "passd: issuer=ExampleGym selected pass uniqueID : \"access-loyalty-generic-12345\""
            ),
            vec![WalletItem::CandidatePass(
                "access-loyalty-generic-12345".into()
            )]
        );
    }
}

#[cfg(test)]
mod literal_identifier_tests {
    use super::*;
    #[test]
    fn absent_or_placeholder_selection_is_not_a_discovered_identifier() {
        for placeholder in [
            "null", "nil", "none", "unknown", "private", "redacted", "...",
        ] {
            let line = format!("passd: selected pass uniqueID : {placeholder}");
            assert!(
                extract_wallet_items(&line).is_empty(),
                "placeholder must not become inventory"
            );
        }
    }
    #[test]
    fn selected_event_preserves_complete_short_dotted_identifier_without_guessing() {
        assert_eq!(
            extract_wallet_items("nfcd: selected pass uniqueID : \"pass.example.42\""),
            vec![WalletItem::CandidatePass("pass.example.42".into())]
        );
        assert!(
            extract_wallet_items("nfcd: selected pass uniqueID : \"partial-pass-123456789 extra\"")
                .is_empty()
        );
        assert!(extract_wallet_items("nfcd: selected pass uniqueID : \"../private\"").is_empty());
        assert!(extract_wallet_items("nfcd: selected pass uniqueID : \"..\"").is_empty());
        assert!(extract_wallet_items("nfcd: selected pass uniqueID : \"not-finished").is_empty());
    }
    #[test]
    fn conflicting_explicit_layouts_are_unsupported_instead_of_guessing() {
        let id = "4FD0Y4djT-oIj--0Odq8GMLVpVo";
        assert_eq!(
            extract_wallet_items(&format!(
                "passd: passType: transit; passKind: access; /var/mobile/Library/Passes/Cards/{id}.pkpass"
            )),
            vec![WalletItem::UnsupportedPass(id.into())]
        );
    }
}

#[cfg(test)]
mod process_context_tests {
    use super::*;
    #[test]
    fn unrelated_loggers_cannot_impersonate_wallet_events_in_message_text() {
        for line in [
            "webserver: received nfcd: passType: access; selected pass uniqueID : \"synthetic-pass-123456789\"",
            "unrelated: quoted PDPassLibrary: wrote pass synthetic-pass-123456789",
            "webserver: Passbook(PassKitUI): Dashboard loading (...): for synthetic-pass-123456789, metadata",
        ] {
            assert!(
                extract_wallet_items(line).is_empty(),
                "unrelated process must not supply inventory"
            );
        }
        assert_eq!(
            extract_wallet_items(
                "Nov  4 12:00:00 ExamplePhone nfcd[123]: selected pass uniqueID : \"synthetic-pass-123456789\""
            ),
            vec![WalletItem::CandidatePass("synthetic-pass-123456789".into())]
        );
    }
}

#[cfg(test)]
mod bounded_path_tests {
    use super::*;
    #[test]
    fn opaque_path_identifiers_remain_unknown_without_layout_evidence() {
        assert_eq!(
            extract_wallet_items(
                "passd: /var/mobile/Library/Passes/Cards/opaque-pass-1234567890.pkpass"
            ),
            vec![WalletItem::CandidatePass("opaque-pass-1234567890".into())]
        );
    }
    #[test]
    fn a_single_record_cannot_create_more_than_256_observations() {
        let line = format!(
            "passd: {}",
            (0..300)
                .map(|index| format!(
                    "/var/mobile/Library/Passes/Cards/synthetic-pass-{index:012}.pkpass"
                ))
                .collect::<Vec<_>>()
                .join(" ")
        );
        assert_eq!(extract_wallet_items(&line).len(), 256);
    }
}

#[cfg(test)]
mod framework_prefix_tests {
    use super::*;
    #[test]
    fn framework_context_is_accepted_only_inside_the_source_prefix() {
        assert_eq!(
            extract_wallet_items(
                "SpringBoard(PassKit): selected pass uniqueID : \"synthetic-pass-123456789\""
            ),
            vec![WalletItem::CandidatePass("synthetic-pass-123456789".into())]
        );
        assert!(extract_wallet_items("webserver: SpringBoard(PassKit): selected pass uniqueID : \"synthetic-pass-123456789\"").is_empty());
    }
}
