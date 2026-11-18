//! API keys: generation, hashing and scopes.
//!
//! Format: `gic_` + 64 hex chars (256 random bits from the OS CSPRNG via UUID v4).
//! Only `sha256(key)` is stored, which is safe for high-entropy secrets.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const KEY_PREFIX: &str = "gic_";
/// Characters of the key kept in plain text so people can recognise it in the UI.
pub const DISPLAY_PREFIX_LEN: usize = 12;
/// A user (manager or admin) may hold at most this many active (not revoked, not expired) keys.
pub const MAX_ACTIVE_KEYS_PER_USER: i64 = 1;

/// What an API key may do. Keys belong to managers or admins and only reach content endpoints;
/// user, key and subject management always require a logged-in browser session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiScope {
    /// Create/edit/delete/publish quizzes and questions, bulk import, upload quiz images.
    #[serde(rename = "quizzes:write")]
    QuizzesWrite,
    /// Upload/replace/delete interactive reviews (and upload images for them).
    #[serde(rename = "reviews:write")]
    ReviewsWrite,
}

impl ApiScope {
    pub const ALL: &'static [ApiScope] = &[ApiScope::QuizzesWrite, ApiScope::ReviewsWrite];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::QuizzesWrite => "quizzes:write",
            Self::ReviewsWrite => "reviews:write",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|sc| sc.as_str() == s)
    }
}

pub fn parse_scopes(stored: &str) -> Vec<ApiScope> {
    stored
        .split(',')
        .filter_map(|s| ApiScope::parse(s.trim()))
        .collect()
}

pub fn join_scopes(scopes: &[ApiScope]) -> String {
    scopes
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

/// A new random key (returned to the user once, never stored).
pub fn generate() -> String {
    let a = uuid::Uuid::new_v4();
    let b = uuid::Uuid::new_v4();
    format!("{KEY_PREFIX}{}{}", a.simple(), b.simple())
}

pub fn hash(key: &str) -> String {
    let digest = Sha256::digest(key.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn looks_like_key(s: &str) -> bool {
    s.len() == KEY_PREFIX.len() + 64
        && s.starts_with(KEY_PREFIX)
        && s[KEY_PREFIX.len()..].bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_keys_are_unique_and_well_formed() {
        let a = generate();
        let b = generate();
        assert_ne!(a, b);
        assert!(looks_like_key(&a));
        assert_eq!(hash(&a).len(), 64);
        assert_ne!(hash(&a), hash(&b));
        assert!(!looks_like_key("gic_short"));
    }

    #[test]
    fn scopes_round_trip() {
        let s = join_scopes(&[ApiScope::QuizzesWrite, ApiScope::ReviewsWrite]);
        assert_eq!(s, "quizzes:write,reviews:write");
        assert_eq!(parse_scopes(&s), ApiScope::ALL);
        assert_eq!(
            parse_scopes("bogus,reviews:write"),
            vec![ApiScope::ReviewsWrite]
        );
    }
}
