//! Small validation helpers shared by the route handlers.

use crate::error::AppError;

/// The 14 Catppuccin accent names, used for subject colors.
pub const ACCENTS: [&str; 14] = [
    "rosewater",
    "flamingo",
    "pink",
    "mauve",
    "red",
    "maroon",
    "peach",
    "yellow",
    "green",
    "teal",
    "sky",
    "sapphire",
    "blue",
    "lavender",
];

/// `lowercase-words-with-dashes`, 1..=100 chars.
pub fn is_slug(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 100
        && !s.starts_with('-')
        && !s.ends_with('-')
        && !s.contains("--")
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

pub fn slugify(s: &str) -> String {
    let mut out = String::new();
    for c in s.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').chars().take(100).collect()
}

/// Trim and turn empty strings into `None`.
pub fn clean(s: Option<String>) -> Option<String> {
    s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

/// An image key produced by our upload endpoint: `quizzes/{id}/{uuid}.{ext}` or `images/{uuid}.{ext}`.
pub fn is_image_key(key: &str) -> bool {
    let (dir, file) = match key.rsplit_once('/') {
        Some(parts) => parts,
        None => return false,
    };
    let dir_ok = dir == "images"
        || dir
            .strip_prefix("quizzes/")
            .is_some_and(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()));
    let Some((stem, ext)) = file.rsplit_once('.') else {
        return false;
    };
    dir_ok
        && matches!(ext, "avif" | "png" | "jpg" | "webp" | "gif")
        && uuid::Uuid::parse_str(stem).is_ok()
}

/// Every uploaded-image key that appears in `text`, e.g. inside a full public URL
/// (`https://cdn.example/images/{uuid}.avif`) or as a bare key. Errs on the side of finding
/// too many (a false match only keeps an extra image alive).
pub fn find_image_keys(text: &str) -> std::collections::BTreeSet<String> {
    let is_key_char = |c: char| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_');
    let mut keys = std::collections::BTreeSet::new();
    for prefix in ["images/", "quizzes/"] {
        for (start, _) in text.match_indices(prefix) {
            let rest = &text[start..];
            let end = rest.find(|c| !is_key_char(c)).unwrap_or(rest.len());
            let candidate = &rest[..end];
            if is_image_key(candidate) {
                keys.insert(candidate.to_string());
            }
        }
    }
    keys
}

/// Validate an optional image key field.
pub fn check_image_key(field: &str, key: &Option<String>) -> Result<(), AppError> {
    match key {
        Some(k) if !is_image_key(k) => Err(AppError::Validation(format!(
            "{field}: not a valid uploaded image key"
        ))),
        _ => Ok(()),
    }
}

pub fn check_len(field: &str, value: &Option<String>, max: usize) -> Result<(), AppError> {
    match value {
        Some(v) if v.chars().count() > max => Err(AppError::Validation(format!(
            "{field}: must be at most {max} characters"
        ))),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert!(is_slug("hci"));
        assert!(is_slug("internet-programming-2"));
        assert!(!is_slug("Bad Slug"));
        assert!(!is_slug("-x"));
        assert_eq!(
            slugify("Human Computer Interaction!"),
            "human-computer-interaction"
        );
    }

    #[test]
    fn image_keys() {
        let id = uuid::Uuid::new_v4();
        assert!(is_image_key(&format!("images/{id}.png")));
        assert!(is_image_key(&format!("quizzes/12/{id}.webp")));
        assert!(is_image_key(&format!("images/{id}.avif")));
        assert!(!is_image_key(&format!("quizzes/x/{id}.png")));
        assert!(!is_image_key(&format!("reviews/1/{id}.html")));
        assert!(!is_image_key("images/../../etc/passwd.png"));
    }

    #[test]
    fn finds_image_keys_in_html() {
        let a = uuid::Uuid::new_v4();
        let b = uuid::Uuid::new_v4();
        let html = format!(
            r#"<img src="https://cdn.example.com/images/{a}.avif"><img src=https://cdn.example.com/quizzes/7/{b}.webp>
               <div style="background:url('images/{a}.avif')"></div><a href="/images/not-a-key.png">x</a>"#
        );
        let keys = find_image_keys(&html);
        assert_eq!(keys.into_iter().collect::<Vec<_>>(), {
            let mut v = vec![format!("images/{a}.avif"), format!("quizzes/7/{b}.webp")];
            v.sort();
            v
        });
    }
}
