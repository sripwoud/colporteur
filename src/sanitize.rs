use ammonia::Builder;
use std::collections::{HashMap, HashSet};

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
    remove_tracking_pixels(&sanitized)
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
}
