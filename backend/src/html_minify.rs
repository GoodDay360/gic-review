//! Minify uploaded interactive-review HTML before it goes to R2.
//!
//! Uses `minify-html` (HTML) with lightningcss (CSS) and oxc (JS). The settings are the
//! conservative, spec-compliant ones so a review behaves exactly as uploaded:
//! - closing tags and the `<html>`/`<head>` opening tags are kept, so the stored file still
//!   passes our own "looks like HTML" upload check if it is downloaded and re-uploaded;
//! - comments are dropped, but SSI and IE conditional comments (bangs) are left alone;
//! - JS is parsed first and only rewritten when it parses cleanly; top-level names are never
//!   mangled, so inline `onclick="fn()"` handlers keep working;
//! - CSS/JS that fails to parse is passed through untouched.
//!
//! Any panic inside the minifier, or an output that is not smaller, falls back to the
//! original bytes: minification can only ever make a review smaller, never break the upload.

use std::panic::{AssertUnwindSafe, catch_unwind};

use minify_html::{Cfg, minify};

fn cfg() -> Cfg {
    Cfg {
        keep_closing_tags: true,
        keep_html_and_head_opening_tags: true,
        keep_input_type_text_attr: true,
        keep_ssi_comments: true,
        keep_comments: false,
        minify_css: true,
        minify_js: true,
        // Template syntax is not expected in reviews, but if present leave it byte-for-byte.
        preserve_brace_template_syntax: true,
        preserve_chevron_percent_template_syntax: true,
        // Everything below stays spec-compliant (all false by default).
        allow_noncompliant_unquoted_attribute_values: false,
        allow_optimal_entities: false,
        allow_removing_spaces_between_attributes: false,
        minify_doctype: false,
        remove_bangs: false,
        remove_processing_instructions: false,
    }
}

/// Minify `html`, returning the original bytes if minification fails or doesn't help.
/// CPU-bound: call through `spawn_blocking`.
pub fn minify_review_html(html: Vec<u8>) -> Vec<u8> {
    let out = match catch_unwind(AssertUnwindSafe(|| minify(&html, &cfg()))) {
        Ok(out) => out,
        Err(_) => {
            tracing::warn!("HTML minifier panicked; storing the review unminified");
            return html;
        }
    };
    if out.is_empty()
        || out.len() >= html.len()
        || std::str::from_utf8(&out).is_err()
        || !String::from_utf8_lossy(&out)
            .to_ascii_lowercase()
            .contains("<html")
    {
        return html;
    }
    tracing::debug!(before = html.len(), after = out.len(), "minified review HTML");
    out
}

/// Async wrapper that runs the minifier on the blocking pool.
pub async fn minify_review_html_async(html: Vec<u8>) -> Vec<u8> {
    let fallback = html.clone();
    tokio::task::spawn_blocking(move || minify_review_html(html))
        .await
        .unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_html_tag_and_shrinks_starter() {
        let src = include_bytes!("../../docs/examples/review-starter.html").to_vec();
        let out = minify_review_html(src.clone());
        assert!(out.len() < src.len());
        let text = String::from_utf8(out).unwrap().to_ascii_lowercase();
        assert!(text.contains("<html"));
        assert!(text.contains("</html>"));
    }

    #[test]
    fn keeps_top_level_names_for_inline_handlers() {
        let src = br#"<!doctype html><html><head><title>t</title></head><body>
            <button onclick="showAnswer()">  Show   answer </button>
            <script>
              function showAnswer() { const answerElement = document.body; answerElement.dataset.shown = "yes"; }
            </script></body></html>"#
            .to_vec();
        let out = String::from_utf8(minify_review_html(src)).unwrap();
        assert!(out.contains("onclick=showAnswer()") || out.contains("onclick=\"showAnswer()\""));
        assert!(out.contains("function showAnswer("));
    }

    #[test]
    fn invalid_js_and_css_pass_through() {
        let src = b"<!doctype html><html><head><style>a{color:red;;;  </style></head><body>\
            <script>let x = ;  // broken\n</script><pre>  keep   spaces  </pre></body></html>"
            .to_vec();
        let out = String::from_utf8(minify_review_html(src)).unwrap();
        assert!(out.contains("let x = ;"));
        assert!(out.contains("<pre>  keep   spaces  </pre>"));
    }
}
