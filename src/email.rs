use chrono::{DateTime, Utc};
use eyre::Context;
use mailparse::{MailHeaderMap, parse_mail};

use crate::sanitize::{sanitize_html, text_to_html};

#[derive(Debug)]
pub struct EmailContent {
    pub subject: String,
    pub from: String,
    pub date: DateTime<Utc>,
    pub message_id: Option<String>,
    pub feed_html: String,
}

fn body_from_parts(html: Option<String>, text: Option<String>) -> eyre::Result<String> {
    if let Some(h) = html {
        return Ok(sanitize_html(&h));
    }
    if let Some(t) = text {
        return Ok(text_to_html(&t));
    }
    Err(eyre::eyre!("email has no text/html or text/plain body"))
}

fn extract_bodies(mail: &mailparse::ParsedMail) -> eyre::Result<(Option<String>, Option<String>)> {
    let mime = mail.ctype.mimetype.as_str();

    if mime == "text/html" {
        let body = mail
            .get_body()
            .wrap_err("failed to decode text/html body")?;
        return Ok((Some(body), None));
    }

    if mime == "text/plain" {
        let body = mail
            .get_body()
            .wrap_err("failed to decode text/plain body")?;
        return Ok((None, Some(body)));
    }

    if mime.starts_with("multipart/") {
        let mut html: Option<String> = None;
        let mut text: Option<String> = None;

        for part in &mail.subparts {
            let part_mime = part.ctype.mimetype.as_str();
            if part_mime == "text/html" && html.is_none() {
                html = Some(
                    part.get_body()
                        .wrap_err("failed to decode text/html part")?,
                );
            } else if part_mime == "text/plain" && text.is_none() {
                text = Some(
                    part.get_body()
                        .wrap_err("failed to decode text/plain part")?,
                );
            } else if part_mime.starts_with("multipart/") {
                let (sub_html, sub_text) = extract_bodies(part)?;
                if html.is_none() {
                    html = sub_html;
                }
                if text.is_none() {
                    text = sub_text;
                }
            }
        }

        return Ok((html, text));
    }

    Ok((None, None))
}

pub fn parse(raw: &[u8]) -> eyre::Result<EmailContent> {
    let mail = parse_mail(raw).wrap_err("failed to parse raw email")?;

    let subject = mail
        .headers
        .get_first_value("Subject")
        .ok_or_else(|| eyre::eyre!("missing Subject header"))?;

    let from = mail
        .headers
        .get_first_value("From")
        .ok_or_else(|| eyre::eyre!("missing From header"))?;

    let date_str = mail
        .headers
        .get_first_value("Date")
        .ok_or_else(|| eyre::eyre!("missing Date header"))?;

    let timestamp = mailparse::dateparse(&date_str).wrap_err("failed to parse Date header")?;

    let date = DateTime::from_timestamp(timestamp, 0)
        .ok_or_else(|| eyre::eyre!("timestamp out of range: {timestamp}"))?;

    let message_id = mail.headers.get_first_value("Message-ID");

    let (html, text) = extract_bodies(&mail)?;

    let feed_html = body_from_parts(html, text)?;

    Ok(EmailContent {
        subject,
        from,
        date,
        message_id,
        feed_html,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    static SAMPLE1: &[u8] = include_bytes!("../tests/fixtures/sample1.eml");
    static SAMPLE2: &[u8] = include_bytes!("../tests/fixtures/sample2.eml");
    static SAMPLE3: &[u8] = include_bytes!("../tests/fixtures/sample3.eml");

    #[test]
    fn parse_sample1_ideabrowser() {
        let email = parse(SAMPLE1).unwrap();
        assert_eq!(email.subject, "Idea of the Day: AI for Functional Medicine");
        assert_eq!(
            email.from,
            "Ideabrowser <notifications@mail.ideabrowser.com>"
        );
        assert_eq!(email.date.to_rfc3339(), "2026-02-24T17:11:14+00:00");
        assert_eq!(
            email.message_id.as_deref(),
            Some(
                "<0100019c90a2308f-d58ec023-533d-4e97-bab2-57c47a94691b-000000@email.amazonses.com>"
            )
        );
        assert!(!email.feed_html.is_empty());
    }

    #[test]
    fn parse_sample2_german() {
        let email = parse(SAMPLE2).unwrap();
        assert_eq!(email.subject, "Gesundheitsportal: News vom 20.02.2026");
        assert_eq!(
            email.from,
            "hansemerkur.gesundheitsportal-privat.de <noreply@gesundheitsportal-privat.de>"
        );
        assert_eq!(email.date.to_rfc3339(), "2026-02-20T05:00:39+00:00");
        assert!(!email.feed_html.is_empty());
    }

    #[test]
    fn parse_sample3_french() {
        let email = parse(SAMPLE3).unwrap();
        assert_eq!(email.subject, "L'interview de ma fille (Joy) de 13 ans !");
        assert_eq!(
            email.from,
            "\"Cool Parents Make Happy Kids\" <coaching@coolparentsmakehappykids.com>"
        );
        assert_eq!(email.date.to_rfc3339(), "2026-02-22T19:03:08+00:00");
        assert!(!email.feed_html.is_empty());
    }

    #[test]
    fn all_samples_have_html_body() {
        for raw in [SAMPLE1, SAMPLE2, SAMPLE3] {
            let email = parse(raw).unwrap();
            assert!(!email.feed_html.is_empty());
        }
    }

    #[test]
    fn body_from_parts_html_wins_over_text() {
        let result = body_from_parts(
            Some("<p>html content</p>".to_string()),
            Some("text content".to_string()),
        )
        .unwrap();
        assert!(result.contains("html content"));
    }

    #[test]
    fn body_from_parts_html_only() {
        let result = body_from_parts(Some("<p>only html</p>".to_string()), None).unwrap();
        assert!(result.contains("only html"));
    }

    #[test]
    fn body_from_parts_text_only() {
        let result = body_from_parts(None, Some("only text".to_string())).unwrap();
        assert!(result.contains("only text"));
        assert!(result.contains("<p>"));
    }

    #[test]
    fn body_from_parts_neither_returns_err() {
        let result = body_from_parts(None, None);
        assert!(result.is_err());
    }
}
