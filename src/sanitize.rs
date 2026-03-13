use ammonia::Builder;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

static NBSP_PADDING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?:(?:&nbsp;|\x{00A0})[\x{200B}-\x{200D}\x{FEFF}]+){3,}|(?:(?:&nbsp;|\x{00A0})\s*){10,}",
    )
    .unwrap()
});

static EXCESSIVE_BR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:<br\s*/?>[\s]*){3,}").unwrap());

static EMPTY_BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<div>\s*(?:(?:&nbsp;|\x{00A0})\s*)*</div>|<p>\s*(?:(?:&nbsp;|\x{00A0})\s*)*</p>")
        .unwrap()
});

static MULTI_NEWLINES_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r">(?:\r?\n){3,}<").unwrap());

pub fn sanitize_html(html: &str) -> String {
    let mut builder = Builder::new();
    builder
        .tags(HashSet::from([
            "p",
            "a",
            "img",
            "ul",
            "ol",
            "li",
            "h1",
            "h2",
            "h3",
            "h4",
            "h5",
            "h6",
            "br",
            "hr",
            "strong",
            "em",
            "b",
            "i",
            "blockquote",
            "pre",
            "code",
            "table",
            "thead",
            "tbody",
            "tr",
            "td",
            "th",
            "div",
            "span",
            "sup",
            "sub",
        ]))
        .clean_content_tags(HashSet::from(["script", "style"]))
        .tag_attributes(HashMap::from([
            ("a", HashSet::from(["href", "title"])),
            ("img", HashSet::from(["src", "alt", "width", "height"])),
            ("td", HashSet::from(["colspan", "rowspan"])),
            ("th", HashSet::from(["colspan", "rowspan"])),
        ]))
        .url_schemes(HashSet::from(["http", "https"]));

    let sanitized = builder.clean(html).to_string();
    let without_pixels = remove_tracking_pixels(&sanitized);
    clean_email_noise(&without_pixels)
}

const MAX_EMPTY_BLOCK_PASSES: usize = 10;

fn clean_email_noise(html: &str) -> String {
    let (work, preserved) = protect_preformatted(html);
    let result = NBSP_PADDING_RE.replace_all(&work, " ");
    let result = EXCESSIVE_BR_RE.replace_all(&result, "<br><br>");
    let mut current = result.into_owned();
    for _ in 0..MAX_EMPTY_BLOCK_PASSES {
        let cleaned = EMPTY_BLOCK_RE.replace_all(&current, "");
        if let std::borrow::Cow::Borrowed(_) = cleaned {
            break;
        }
        current = cleaned.into_owned();
    }
    let result = MULTI_NEWLINES_RE.replace_all(&current, ">\n\n<");
    let trimmed = result.trim().to_string();
    restore_preformatted(&trimmed, &preserved)
}

fn protect_preformatted(html: &str) -> (String, Vec<String>) {
    let mut result = html.to_string();
    let mut blocks = Vec::new();
    for tag in ["pre", "code"] {
        let open = format!("<{tag}");
        let close = format!("</{tag}>");
        let mut pos = 0;
        while let Some(start) = result[pos..].find(&open) {
            let abs_start = pos + start;
            if let Some(end_rel) = result[abs_start..].find(&close) {
                let abs_end = abs_start + end_rel + close.len();
                let placeholder = format!("\x00COLPORTEUR_{}\x00", blocks.len());
                blocks.push(result[abs_start..abs_end].to_string());
                result.replace_range(abs_start..abs_end, &placeholder);
                pos = abs_start + placeholder.len();
            } else {
                break;
            }
        }
    }
    (result, blocks)
}

fn restore_preformatted(html: &str, blocks: &[String]) -> String {
    let mut result = html.to_string();
    for (i, block) in blocks.iter().enumerate() {
        let placeholder = format!("\x00COLPORTEUR_{i}\x00");
        result = result.replace(&placeholder, block);
    }
    result
}

fn remove_tracking_pixels(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while let Some(img_start) = remaining.find("<img") {
        let before = &remaining[..img_start];
        let after_open = &remaining[img_start..];

        let tag_end = after_open.find('>').map(|i| i + 1);
        match tag_end {
            None => {
                result.push_str(remaining);
                return result;
            }
            Some(end) => {
                let tag = &after_open[..end];
                if is_tracking_pixel(tag) {
                    result.push_str(before);
                } else {
                    result.push_str(before);
                    result.push_str(tag);
                }
                remaining = &after_open[end..];
            }
        }
    }

    result.push_str(remaining);
    result
}

fn is_tracking_pixel(tag: &str) -> bool {
    has_attr_value(tag, "width", "1") && has_attr_value(tag, "height", "1")
}

fn has_attr_value(tag: &str, attr: &str, value: &str) -> bool {
    let double_quoted = format!("{attr}=\"{value}\"");
    let single_quoted = format!("{attr}='{value}'");
    let unquoted = format!("{attr}={value}");
    tag.contains(&double_quoted) || tag.contains(&single_quoted) || tag.contains(&unquoted)
}

pub fn text_to_html(text: &str) -> String {
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let with_paragraphs = escaped.replace("\n\n", "</p><p>");
    let with_breaks = with_paragraphs.replace('\n', "<br>");
    format!("<p>{with_breaks}</p>")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_script_tags() {
        let result = sanitize_html("<p>hello</p><script>alert(1)</script>");
        assert_eq!(result, "<p>hello</p>");
    }

    #[test]
    fn strips_style_tags() {
        let result = sanitize_html("<p>hello</p><style>body{color:red}</style>");
        assert_eq!(result, "<p>hello</p>");
    }

    #[test]
    fn removes_tracking_pixels() {
        let result = sanitize_html(
            r#"<img width="1" height="1" src="https://track.example.com/pixel.gif">"#,
        );
        assert_eq!(result, "");
    }

    #[test]
    fn keeps_content_images() {
        let result = sanitize_html(r#"<img src="https://example.com/photo.jpg" alt="photo">"#);
        assert!(result.contains("https://example.com/photo.jpg"));
        assert!(result.contains("photo"));
    }

    #[test]
    fn keeps_links() {
        let result = sanitize_html(r#"<a href="https://example.com">click</a>"#);
        assert!(result.contains("https://example.com"));
        assert!(result.contains("click"));
    }

    #[test]
    fn text_to_html_escapes_and_wraps() {
        let result = text_to_html("hello & world\nfoo\n\nbar");
        assert!(result.starts_with("<p>"));
        assert!(result.ends_with("</p>"));
        assert!(result.contains("&amp;"));
        assert!(result.contains("<br>"));
        assert!(result.contains("</p><p>"));
    }

    #[test]
    fn strips_preheader_padding() {
        let padding = "&nbsp;\u{200C}".repeat(50);
        let html = format!("<div>{padding}</div><p>real content</p>");
        let result = clean_email_noise(&html);
        assert!(
            !result.contains("&nbsp;\u{200C}&nbsp;"),
            "preheader padding should be stripped"
        );
        assert!(result.contains("real content"));
    }

    #[test]
    fn preserves_short_nbsp_runs() {
        let html = "<p>hello&nbsp;&nbsp;world</p>";
        let result = clean_email_noise(html);
        assert_eq!(result, html);
    }

    #[test]
    fn strips_ten_or_more_plain_nbsp() {
        let nbsp_10 = "&nbsp;".repeat(10);
        let html = format!("<p>{nbsp_10}text</p>");
        let result = clean_email_noise(&html);
        assert!(
            !result.contains("&nbsp;&nbsp;"),
            "10+ plain NBSP should be stripped"
        );
    }

    #[test]
    fn preserves_nine_plain_nbsp() {
        let nbsp_9 = "&nbsp;".repeat(9);
        let html = format!("<p>{nbsp_9}text</p>");
        let result = clean_email_noise(&html);
        assert_eq!(result, html);
    }

    #[test]
    fn preserves_nbsp_indentation_in_pre() {
        let nbsp_12 = "&nbsp;".repeat(12);
        let html = format!("<pre>{nbsp_12}indent</pre>");
        let result = clean_email_noise(&html);
        assert_eq!(result, html);
    }

    #[test]
    fn preserves_br_inside_pre() {
        let html = "<pre>a<br><br><br><br>b</pre>";
        let result = clean_email_noise(html);
        assert_eq!(result, html);
    }

    #[test]
    fn preserves_code_block_content() {
        let nbsp_15 = "&nbsp;".repeat(15);
        let html = format!("<code>{nbsp_15}indented</code><p>text</p>");
        let result = clean_email_noise(&html);
        assert!(
            result.contains(&nbsp_15),
            "code block content must be preserved"
        );
    }

    #[test]
    fn collapses_excessive_br() {
        let html = "<p>one</p><br><br><br><br><br><p>two</p>";
        let result = clean_email_noise(html);
        assert_eq!(result, "<p>one</p><br><br><p>two</p>");
    }

    #[test]
    fn removes_empty_div_with_nbsp() {
        let html = "<div>&nbsp;</div><p>content</p>";
        let result = clean_email_noise(html);
        assert_eq!(result, "<p>content</p>");
    }

    #[test]
    fn removes_div_with_multiple_nbsp() {
        let html = "<div>&nbsp;&nbsp;&nbsp;</div><p>content</p>";
        let result = clean_email_noise(html);
        assert_eq!(result, "<p>content</p>");
    }

    #[test]
    fn removes_empty_div() {
        let html = "<div>  </div><p>content</p>";
        let result = clean_email_noise(html);
        assert_eq!(result, "<p>content</p>");
    }

    #[test]
    fn removes_empty_p() {
        let html = "<p></p><p>content</p>";
        let result = clean_email_noise(html);
        assert_eq!(result, "<p>content</p>");
    }

    #[test]
    fn removes_nested_empty_blocks() {
        let html = "<div><p></p></div><p>content</p>";
        let result = clean_email_noise(html);
        assert!(
            !result.contains("<div>"),
            "outer div should be removed after inner p"
        );
        assert_eq!(result, "<p>content</p>");
    }

    #[test]
    fn strips_short_nbsp_with_zero_width_chars() {
        let padding = format!("&nbsp;\u{200C}&nbsp;\u{200C}&nbsp;\u{200C}");
        let html = format!("<div>{padding}</div><p>content</p>");
        let result = clean_email_noise(&html);
        assert!(
            !result.contains("&nbsp;\u{200C}&nbsp;"),
            "NBSP+ZW runs should be stripped even at 3 repetitions"
        );
    }

    #[test]
    fn preserves_div_with_content() {
        let html = "<div>hello world</div>";
        let result = clean_email_noise(html);
        assert_eq!(result, html);
    }

    #[test]
    fn collapses_multi_newlines_between_tags() {
        let html = "<p>one</p>\n\n\n\n\n<p>two</p>";
        let result = clean_email_noise(html);
        assert_eq!(result, "<p>one</p>\n\n<p>two</p>");
    }

    #[test]
    fn collapses_crlf_newlines_between_tags() {
        let html = "<p>one</p>\r\n\r\n\r\n\r\n<p>two</p>";
        let result = clean_email_noise(html);
        assert_eq!(result, "<p>one</p>\n\n<p>two</p>");
    }

    #[test]
    fn preserves_newlines_inside_pre() {
        let html = "<pre>line1\n\n\n\nline4</pre>";
        let result = clean_email_noise(html);
        assert_eq!(result, html);
    }

    #[test]
    fn trims_leading_trailing_whitespace() {
        let html = "\n\n  <p>content</p>  \n\n";
        let result = clean_email_noise(html);
        assert_eq!(result, "<p>content</p>");
    }

    #[test]
    fn tracking_pixel_removal_then_empty_block_cleanup() {
        let html = r#"<div><img width="1" height="1" src="https://track.example.com/pixel.gif"></div><p>content</p>"#;
        let result = sanitize_html(html);
        assert!(
            !result.contains("<div></div>"),
            "empty wrapper after pixel removal should be cleaned"
        );
        assert!(result.contains("content"));
    }

    #[test]
    fn restore_preformatted_no_collision_with_10_plus_blocks() {
        let blocks: Vec<String> = (0..12).map(|i| format!("<code>block{i}</code>")).collect();
        let mut html = String::new();
        for i in 0..12 {
            html.push_str(&format!("\x00COLPORTEUR_{i}\x00"));
        }
        let result = restore_preformatted(&html, &blocks);
        for i in 0..12 {
            assert!(
                result.contains(&format!("<code>block{i}</code>")),
                "block {i} should be restored correctly"
            );
        }
        assert!(!result.contains("COLPORTEUR_"));
    }

    #[test]
    fn clean_email_noise_integration() {
        let padding = "&nbsp;\u{200C}".repeat(100);
        let html = format!(
            "\n  <div>{padding}</div>\n<p>real</p><br><br><br><br><div>&nbsp;</div><p></p><p>end</p>\n\n"
        );
        let result = clean_email_noise(&html);
        assert!(!result.contains("&nbsp;\u{200C}&nbsp;"));
        assert!(result.contains("<p>real</p>"));
        assert!(result.contains("<p>end</p>"));
        assert!(!result.contains("<p></p>"));
        assert!(!result.contains("<div>&nbsp;</div>"));
        let br_count = result.matches("<br>").count();
        assert!(br_count <= 2, "expected at most 2 <br>, got {br_count}");
    }
}
