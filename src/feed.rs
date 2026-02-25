use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{BufReader, Write};
use std::path::Path;

use atom_syndication::{Content, Entry, Feed, Generator, Person, Text};
use chrono::Utc;
use eyre::Context;

use crate::email::EmailContent;

pub fn load_or_create(path: &Path, title: &str) -> eyre::Result<Feed> {
    if path.exists() {
        let content = std::fs::read_to_string(path)
            .wrap_err_with(|| format!("failed to read feed file: {}", path.display()))?;
        let feed = Feed::read_from(BufReader::new(content.as_bytes()))
            .wrap_err("failed to parse atom feed")?;
        return Ok(feed);
    }

    let slug = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();

    let feed = Feed {
        title: Text::plain(title),
        id: format!("urn:colporteur:feed:{slug}"),
        updated: Utc::now().fixed_offset(),
        generator: Some(Generator {
            value: "colporteur".into(),
            ..Default::default()
        }),
        ..Default::default()
    };

    Ok(feed)
}

pub fn append_entry(feed: &mut Feed, email: &EmailContent, sanitized_html: &str) {
    let entry = Entry {
        id: entry_id(email),
        title: Text::plain(&email.subject),
        updated: email.date.fixed_offset(),
        authors: vec![Person {
            name: email.from.clone(),
            ..Default::default()
        }],
        content: Some(Content {
            content_type: Some("html".to_string()),
            value: Some(sanitized_html.to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut entries = vec![entry];
    entries.append(&mut feed.entries);
    feed.entries = entries;
    feed.updated = Utc::now().fixed_offset();
}

pub fn trim_entries(feed: &mut Feed, max: usize) {
    if feed.entries.len() > max {
        feed.entries.truncate(max);
    }
}

pub fn write_atomic(feed: &Feed, path: &Path) -> eyre::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .wrap_err_with(|| format!("failed to create directories for {}", path.display()))?;
    }

    let tmp_path = path.with_extension("tmp");

    let mut file = std::fs::File::create(&tmp_path)
        .wrap_err_with(|| format!("failed to create tmp file: {}", tmp_path.display()))?;

    let xml = feed.to_string();
    file.write_all(xml.as_bytes())
        .wrap_err("failed to write feed xml")?;

    std::fs::rename(&tmp_path, path).wrap_err_with(|| {
        format!(
            "failed to rename {} to {}",
            tmp_path.display(),
            path.display()
        )
    })?;

    Ok(())
}

fn entry_id(email: &EmailContent) -> String {
    if let Some(ref mid) = email.message_id {
        return mid.trim_matches(|c| c == '<' || c == '>').to_string();
    }

    let mut hasher = DefaultHasher::new();
    email.from.hash(&mut hasher);
    email.date.hash(&mut hasher);
    email.subject.hash(&mut hasher);
    let hash = hasher.finish();

    format!("urn:colporteur:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::*;

    fn make_email(subject: &str, from: &str, message_id: Option<&str>) -> EmailContent {
        EmailContent {
            subject: subject.to_string(),
            from: from.to_string(),
            date: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
            message_id: message_id.map(str::to_string),
            html: Some("<p>hello</p>".to_string()),
            text: None,
        }
    }

    fn tmp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("colporteur_test_{name}_{}.xml", std::process::id()))
    }

    #[test]
    fn create_new_feed_has_correct_title() {
        let path = tmp_path("create_new");
        let feed = load_or_create(&path, "My Newsletter").unwrap();
        assert_eq!(feed.title.as_str(), "My Newsletter");
    }

    #[test]
    fn append_entry_increases_count() {
        let path = tmp_path("append_entry");
        let mut feed = load_or_create(&path, "Test Feed").unwrap();
        let email = make_email("Hello", "sender@example.com", None);
        append_entry(&mut feed, &email, "<p>hello</p>");
        assert_eq!(feed.entries().len(), 1);
    }

    #[test]
    fn trim_entries_keeps_max() {
        let path = tmp_path("trim_entries");
        let mut feed = load_or_create(&path, "Trim Feed").unwrap();

        for i in 0..5 {
            let email = make_email(&format!("Subject {i}"), "sender@example.com", None);
            append_entry(&mut feed, &email, "<p>body</p>");
        }

        assert_eq!(feed.entries().len(), 5);
        trim_entries(&mut feed, 2);
        assert_eq!(feed.entries().len(), 2);
        assert_eq!(feed.entries()[0].title.as_str(), "Subject 4");
        assert_eq!(feed.entries()[1].title.as_str(), "Subject 3");
    }

    #[test]
    fn write_and_load_round_trip() {
        let path = tmp_path("round_trip");
        let mut feed = load_or_create(&path, "Round Trip Feed").unwrap();

        let email = make_email("Round Trip Subject", "rt@example.com", None);
        append_entry(&mut feed, &email, "<p>rt</p>");

        write_atomic(&feed, &path).unwrap();

        let loaded = load_or_create(&path, "ignored").unwrap();
        assert_eq!(loaded.title.as_str(), "Round Trip Feed");
        assert_eq!(loaded.entries().len(), 1);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn entry_id_uses_message_id_when_present() {
        let email = make_email(
            "Subj",
            "from@example.com",
            Some("<abc123@mail.example.com>"),
        );
        let id = entry_id(&email);
        assert_eq!(id, "abc123@mail.example.com");
    }

    #[test]
    fn entry_id_uses_hash_when_no_message_id() {
        let email = make_email("Subj", "from@example.com", None);
        let id = entry_id(&email);
        assert!(
            id.starts_with("urn:colporteur:"),
            "expected urn prefix, got: {id}"
        );
    }
}
