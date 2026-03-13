use crate::config::Config;

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn generate(config: &Config, base_url: &str) -> eyre::Result<String> {
    if base_url.contains('?') || base_url.contains('#') {
        eyre::bail!("base URL must not contain query string or fragment: {base_url}");
    }
    let base = base_url.trim_end_matches('/');

    let mut keys: Vec<&String> = config.feeds.keys().collect();
    keys.sort();

    let outlines: Vec<String> = keys
        .iter()
        .map(|key| {
            let feed = &config.feeds[*key];
            let title = escape_xml(&feed.title);
            let url = escape_xml(&format!("{base}/{key}.xml"));
            format!(
                "    <outline text=\"{title}\" title=\"{title}\" xmlUrl=\"{url}\" type=\"rss\"/>"
            )
        })
        .collect();

    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <opml version=\"2.0\">\n\
         \x20 <head>\n\
         \x20   <title>Colporteur Feeds</title>\n\
         \x20 </head>\n\
         \x20 <body>\n\
         {}\n\
         \x20 </body>\n\
         </opml>\n",
        outlines.join("\n")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AccountConfig, Config, FeedConfig};
    use std::collections::HashMap;

    fn test_config(feeds: Vec<(&str, &str)>) -> Config {
        let mut feed_map = HashMap::new();
        for (key, title) in feeds {
            feed_map.insert(
                key.to_string(),
                FeedConfig {
                    title: title.to_string(),
                    account: "test".to_string(),
                    senders: vec!["a@b.com".to_string()],
                    max_entries: None,
                    url: None,
                },
            );
        }
        let mut accounts = HashMap::new();
        accounts.insert(
            "test".to_string(),
            AccountConfig {
                server: "imap.test.com".to_string(),
                username: "user@test.com".to_string(),
                password: "pass".to_string(),
                mailbox: "INBOX".to_string(),
            },
        );
        Config {
            output_dir: "/tmp/feeds".to_string(),
            max_entries: 50,
            base_url: None,
            accounts,
            feeds: feed_map,
        }
    }

    #[test]
    fn generates_valid_opml_with_single_feed() {
        let config = test_config(vec![("newsletter", "My Newsletter")]);
        let opml = generate(&config, "http://localhost:8085").unwrap();

        assert!(opml.starts_with("<?xml version=\"1.0\""));
        assert!(opml.contains("<opml version=\"2.0\">"));
        assert!(opml.contains("text=\"My Newsletter\""));
        assert!(opml.contains("xmlUrl=\"http://localhost:8085/newsletter.xml\""));
        assert!(opml.contains("type=\"rss\""));
        assert!(opml.contains("</opml>"));
    }

    #[test]
    fn generates_feeds_in_sorted_order() {
        let config = test_config(vec![("zebra", "Zebra Feed"), ("alpha", "Alpha Feed")]);
        let opml = generate(&config, "https://feeds.example.com").unwrap();

        let alpha_pos = opml.find("alpha.xml").unwrap();
        let zebra_pos = opml.find("zebra.xml").unwrap();
        assert!(alpha_pos < zebra_pos);
    }

    #[test]
    fn escapes_xml_special_characters_in_title() {
        let config = test_config(vec![("test", "Tom & Jerry's <News>")]);
        let opml = generate(&config, "http://localhost:8085").unwrap();

        assert!(opml.contains("Tom &amp; Jerry&apos;s &lt;News&gt;"));
    }

    #[test]
    fn strips_trailing_slash_from_base_url() {
        let config = test_config(vec![("feed", "Feed")]);
        let opml = generate(&config, "http://localhost:8085/").unwrap();

        assert!(opml.contains("http://localhost:8085/feed.xml"));
        assert!(!opml.contains("http://localhost:8085//feed.xml"));
    }

    #[test]
    fn rejects_base_url_with_query_string() {
        let config = test_config(vec![("feed", "Feed")]);
        let result = generate(&config, "http://localhost:8085/path?a=1&b=2");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_base_url_with_fragment() {
        let config = test_config(vec![("feed", "Feed")]);
        let result = generate(&config, "http://localhost:8085/path#section");
        assert!(result.is_err());
    }

    #[test]
    fn handles_empty_feeds() {
        let config = test_config(vec![]);
        let opml = generate(&config, "http://localhost:8085").unwrap();

        assert!(opml.contains("<body>"));
        assert!(opml.contains("</body>"));
        assert!(!opml.contains("<outline"));
    }
}
