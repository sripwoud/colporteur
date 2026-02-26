use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eyre::{Context, bail};
use serde::Deserialize;

pub const SAMPLE_CONFIG: &str = r#"# colporteur configuration
# See: https://colporteur.sripwoud.xyz/#/configuration

# Directory where Atom feed files are written
output_dir = "/srv/feeds"

# Maximum entries per feed (default: 50)
# max_entries = 50

# IMAP accounts
[accounts.example]
server = "imap.example.com"
username = "newsletters@example.com"
password_env = "IMAP_EXAMPLE_PASSWORD"  # reads password from this env var
# mailbox = "INBOX"  # default

# Feeds (one per newsletter or group of senders)
[feeds.my-newsletter]
title = "My Newsletter"
account = "example"
senders = ["hello@newsletter.com"]
# max_entries = 25  # override per feed
"#;

fn default_mailbox() -> String {
    "INBOX".to_string()
}

fn default_max_entries() -> usize {
    50
}

#[derive(Debug, Deserialize, Clone)]
pub struct AccountConfig {
    pub server: String,
    pub username: String,
    pub password_env: String,
    #[serde(default = "default_mailbox")]
    pub mailbox: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FeedConfig {
    pub title: String,
    pub account: String,
    pub senders: Vec<String>,
    pub max_entries: Option<usize>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub output_dir: String,
    #[serde(default = "default_max_entries")]
    pub max_entries: usize,
    pub accounts: HashMap<String, AccountConfig>,
    pub feeds: HashMap<String, FeedConfig>,
}

impl AccountConfig {
    pub fn resolve_password(&self) -> eyre::Result<String> {
        std::env::var(&self.password_env)
            .wrap_err_with(|| format!("env var '{}' not set", self.password_env))
    }
}

impl Config {
    pub fn default_path() -> eyre::Result<PathBuf> {
        let dir = dirs::config_dir()
            .ok_or_else(|| eyre::eyre!("could not determine config directory"))?;
        Ok(dir.join("colporteur/config.toml"))
    }

    pub fn load() -> eyre::Result<Self> {
        let path = Self::default_path()?;
        Self::load_from(&path)
    }

    pub fn load_from(path: &Path) -> eyre::Result<Self> {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                bail!(
                    "config file not found: {}\n\n  Run 'colporteur init' to create a sample config.",
                    path.display()
                );
            }
            Err(e) => {
                bail!("cannot read config file {}: {e}", path.display());
            }
        };
        let config: Self = toml::from_str(&content).wrap_err("failed to parse config TOML")?;
        config.validate()?;
        Ok(config)
    }

    pub fn init() -> eyre::Result<PathBuf> {
        let path = Self::default_path()?;
        if path.exists() {
            bail!(
                "config file already exists: {}\n  Edit it directly or remove it first.",
                path.display()
            );
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .wrap_err_with(|| format!("failed to create {}", parent.display()))?;
        }
        std::fs::write(&path, SAMPLE_CONFIG)
            .wrap_err_with(|| format!("failed to write {}", path.display()))?;
        Ok(path)
    }

    pub fn validate(&self) -> eyre::Result<()> {
        let mut errors: Vec<String> = Vec::new();

        if self.output_dir.is_empty() {
            errors.push("output_dir must not be empty".to_string());
        }

        if self.accounts.is_empty() {
            errors.push("at least one account must be defined".to_string());
        }

        for (feed_key, feed) in &self.feeds {
            if !self.accounts.contains_key(&feed.account) {
                errors.push(format!(
                    "feed '{}' references unknown account '{}'",
                    feed_key, feed.account
                ));
            }
        }

        if !errors.is_empty() {
            bail!("config validation failed:\n{}", errors.join("\n"));
        }

        Ok(())
    }

    pub fn max_entries_for(&self, feed_key: &str) -> usize {
        self.feeds
            .get(feed_key)
            .and_then(|f| f.max_entries)
            .unwrap_or(self.max_entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_TOML: &str = r#"
output_dir = "/srv/feeds"
max_entries = 50

[accounts.mxroute]
server = "mail.mxroute.com"
username = "news@domain.com"
password_env = "IMAP_MXROUTE_PASSWORD"
mailbox = "INBOX"

[accounts.gmail]
server = "imap.gmail.com"
username = "user@gmail.com"
password_env = "IMAP_GMAIL_PASSWORD"

[feeds.ideabrowser]
title = "Ideabrowser Daily"
account = "mxroute"
senders = ["notifications@mail.ideabrowser.com"]

[feeds.newsletter]
title = "Some Newsletter"
account = "gmail"
senders = ["sender@newsletter.com"]
max_entries = 10
"#;

    fn parse(toml: &str) -> Config {
        toml::from_str(toml).expect("failed to parse TOML")
    }

    #[test]
    fn parses_valid_toml_with_two_accounts_and_two_feeds() {
        let config = parse(VALID_TOML);

        assert_eq!(config.output_dir, "/srv/feeds");
        assert_eq!(config.max_entries, 50);

        let mxroute = config
            .accounts
            .get("mxroute")
            .expect("mxroute account missing");
        assert_eq!(mxroute.server, "mail.mxroute.com");
        assert_eq!(mxroute.username, "news@domain.com");
        assert_eq!(mxroute.password_env, "IMAP_MXROUTE_PASSWORD");
        assert_eq!(mxroute.mailbox, "INBOX");

        let gmail = config.accounts.get("gmail").expect("gmail account missing");
        assert_eq!(gmail.server, "imap.gmail.com");

        let ideabrowser = config
            .feeds
            .get("ideabrowser")
            .expect("ideabrowser feed missing");
        assert_eq!(ideabrowser.title, "Ideabrowser Daily");
        assert_eq!(ideabrowser.account, "mxroute");
        assert_eq!(
            ideabrowser.senders,
            vec!["notifications@mail.ideabrowser.com"]
        );

        let newsletter = config
            .feeds
            .get("newsletter")
            .expect("newsletter feed missing");
        assert_eq!(newsletter.max_entries, Some(10));
    }

    #[test]
    fn validate_catches_unknown_account_reference() {
        let toml = r#"
output_dir = "/srv/feeds"

[accounts.real]
server = "mail.example.com"
username = "user@example.com"
password_env = "PASS"

[feeds.broken]
title = "Broken Feed"
account = "nonexistent"
senders = ["x@example.com"]
"#;
        let config = parse(toml);
        let result = config.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("nonexistent"),
            "expected 'nonexistent' in error: {msg}"
        );
    }

    #[test]
    fn resolve_password_reads_env_var() {
        unsafe { std::env::set_var("TEST_IMAP_PASS_CONFIG", "s3cr3t") };
        let account = AccountConfig {
            server: "mail.example.com".to_string(),
            username: "user@example.com".to_string(),
            password_env: "TEST_IMAP_PASS_CONFIG".to_string(),
            mailbox: "INBOX".to_string(),
        };
        assert_eq!(account.resolve_password().unwrap(), "s3cr3t");
    }

    #[test]
    fn default_mailbox_is_inbox_when_omitted() {
        let toml = r#"
output_dir = "/srv/feeds"

[accounts.minimal]
server = "mail.example.com"
username = "user@example.com"
password_env = "PASS"

[feeds.f]
title = "Feed"
account = "minimal"
senders = ["a@example.com"]
"#;
        let config = parse(toml);
        assert_eq!(config.accounts["minimal"].mailbox, "INBOX");
    }

    #[test]
    fn default_max_entries_is_50_when_omitted() {
        let toml = r#"
output_dir = "/srv/feeds"

[accounts.a]
server = "s"
username = "u"
password_env = "P"

[feeds.f]
title = "F"
account = "a"
senders = ["x@x.com"]
"#;
        let config = parse(toml);
        assert_eq!(config.max_entries, 50);
        assert!(config.feeds["f"].max_entries.is_none());
    }

    #[test]
    fn max_entries_for_returns_feed_level_override() {
        let config = parse(VALID_TOML);
        assert_eq!(config.max_entries_for("newsletter"), 10);
        assert_eq!(config.max_entries_for("ideabrowser"), 50);
    }

    #[test]
    fn load_from_missing_file_suggests_init() {
        let path = Path::new("/tmp/colporteur-test-nonexistent/config.toml");
        let err = Config::load_from(path).unwrap_err().to_string();
        assert!(
            err.contains("colporteur init"),
            "expected init suggestion in: {err}"
        );
    }

    #[test]
    fn load_from_invalid_toml_reports_parse_error() {
        let dir = std::env::temp_dir().join("colporteur-test-invalid-toml");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(&path, "not valid [[[toml").unwrap();
        let err = Config::load_from(&path).unwrap_err().to_string();
        std::fs::remove_dir_all(&dir).ok();
        assert!(err.contains("parse"), "expected parse error in: {err}");
    }

    #[test]
    fn sample_config_is_valid_toml() {
        let _: toml::Value =
            toml::from_str(SAMPLE_CONFIG).expect("SAMPLE_CONFIG must be valid TOML");
    }
}
