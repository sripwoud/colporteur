use std::collections::HashMap;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::{Command, Stdio};

use eyre::{Context, bail};
use serde::Deserialize;

pub const SAMPLE_CONFIG: &str = r#"# colporteur configuration
# See: https://colporteur.sripwoud.xyz/#/configuration

# Directory where Atom feed files are written
output_dir = "/var/lib/colporteur/feeds"

# Maximum entries per feed (default: 50)
# max_entries = 50

# IMAP accounts
[accounts.example]
server = "imap.example.com"
username = "newsletters@example.com"
password = "your-imap-password"
# password = "!pass show email/imap"  # or use a command (! prefix)
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

#[derive(Deserialize, Clone)]
pub struct AccountConfig {
    pub server: String,
    pub username: String,
    pub password: String,
    #[serde(default = "default_mailbox")]
    pub mailbox: String,
}

impl std::fmt::Debug for AccountConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccountConfig")
            .field("server", &self.server)
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .field("mailbox", &self.mailbox)
            .finish()
    }
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
        if let Some(rest) = self.password.strip_prefix("!!") {
            return Ok(format!("!{rest}"));
        }

        if let Some(cmd) = self.password.strip_prefix('!') {
            return Self::run_password_command(cmd);
        }

        Ok(self.password.clone())
    }

    #[cfg(not(unix))]
    fn run_password_command(cmd: &str) -> eyre::Result<String> {
        bail!("command-based passwords are only supported on Unix: {cmd}");
    }

    #[cfg(unix)]
    fn run_password_command(cmd: &str) -> eyre::Result<String> {
        let output = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .wrap_err_with(|| format!("failed to execute password command: {cmd}"))?;

        if !output.status.success() {
            let code = output
                .status
                .code()
                .map_or("signal".to_string(), |c| c.to_string());
            bail!("password command failed (exit {code}): {cmd}");
        }

        if !output.stderr.is_empty() {
            log::debug!(
                "password command produced output on stderr ({} bytes)",
                output.stderr.len()
            );
        }

        let password = String::from_utf8(output.stdout)
            .wrap_err_with(|| format!("password command output is not valid UTF-8: {cmd}"))?
            .trim()
            .to_string();

        if password.is_empty() {
            bail!("password command produced no output: {cmd}");
        }

        Ok(password)
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
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .wrap_err_with(|| format!("failed to create {}", parent.display()))?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
                .and_then(|mut f| std::io::Write::write_all(&mut f, SAMPLE_CONFIG.as_bytes()))
                .map_err(|e| {
                    if e.kind() == std::io::ErrorKind::AlreadyExists {
                        eyre::eyre!(
                            "config file already exists: {}\n  Edit it directly or remove it first.",
                            path.display()
                        )
                    } else {
                        eyre::eyre!("failed to write {}: {e}", path.display())
                    }
                })?;
        }
        #[cfg(not(unix))]
        {
            if path.exists() {
                bail!(
                    "config file already exists: {}\n  Edit it directly or remove it first.",
                    path.display()
                );
            }
            std::fs::write(&path, SAMPLE_CONFIG)
                .wrap_err_with(|| format!("failed to write {}", path.display()))?;
        }
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
output_dir = "/var/lib/colporteur/feeds"
max_entries = 50

[accounts.mxroute]
server = "mail.mxroute.com"
username = "news@domain.com"
password = "secret1"
mailbox = "INBOX"

[accounts.gmail]
server = "imap.gmail.com"
username = "user@gmail.com"
password = "secret2"

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

        assert_eq!(config.output_dir, "/var/lib/colporteur/feeds");
        assert_eq!(config.max_entries, 50);

        let mxroute = config
            .accounts
            .get("mxroute")
            .expect("mxroute account missing");
        assert_eq!(mxroute.server, "mail.mxroute.com");
        assert_eq!(mxroute.username, "news@domain.com");
        assert_eq!(mxroute.password, "secret1");
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
output_dir = "/var/lib/colporteur/feeds"

[accounts.real]
server = "mail.example.com"
username = "user@example.com"
password = "pass"

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

    fn account_with_password(password: &str) -> AccountConfig {
        AccountConfig {
            server: "mail.example.com".to_string(),
            username: "user@example.com".to_string(),
            password: password.to_string(),
            mailbox: "INBOX".to_string(),
        }
    }

    #[test]
    fn resolve_plaintext_password() {
        assert_eq!(
            account_with_password("s3cr3t").resolve_password().unwrap(),
            "s3cr3t"
        );
    }

    #[test]
    #[cfg(unix)]
    fn resolve_command_password() {
        assert_eq!(
            account_with_password("!echo secret123")
                .resolve_password()
                .unwrap(),
            "secret123"
        );
    }

    #[test]
    #[cfg(unix)]
    fn resolve_command_trims_whitespace() {
        assert_eq!(
            account_with_password("!echo '  secret  '")
                .resolve_password()
                .unwrap(),
            "secret"
        );
    }

    #[test]
    fn resolve_escape_literal_bang() {
        assert_eq!(
            account_with_password("!!not-a-command")
                .resolve_password()
                .unwrap(),
            "!not-a-command"
        );
    }

    #[test]
    #[cfg(unix)]
    fn resolve_command_failure() {
        let err = account_with_password("!false")
            .resolve_password()
            .unwrap_err()
            .to_string();
        assert!(err.contains("exit"), "expected exit info in: {err}");
    }

    #[test]
    #[cfg(unix)]
    fn resolve_command_empty_output() {
        let err = account_with_password("!echo -n ''")
            .resolve_password()
            .unwrap_err()
            .to_string();
        assert!(err.contains("no output"), "expected 'no output' in: {err}");
    }

    #[test]
    #[cfg(unix)]
    fn resolve_command_not_found() {
        let err = account_with_password("!nonexistent_binary_xyz_99")
            .resolve_password()
            .unwrap_err()
            .to_string();
        assert!(err.contains("nonexistent_binary_xyz_99"));
    }

    #[test]
    fn default_mailbox_is_inbox_when_omitted() {
        let toml = r#"
output_dir = "/var/lib/colporteur/feeds"

[accounts.minimal]
server = "mail.example.com"
username = "user@example.com"
password = "pass"

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
output_dir = "/var/lib/colporteur/feeds"

[accounts.a]
server = "s"
username = "u"
password = "p"

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

    #[test]
    fn debug_output_redacts_password() {
        let account = AccountConfig {
            server: "mail.example.com".to_string(),
            username: "user@example.com".to_string(),
            password: "super-secret".to_string(),
            mailbox: "INBOX".to_string(),
        };
        let debug = format!("{:?}", account);
        assert!(
            !debug.contains("super-secret"),
            "password leaked in debug output: {debug}"
        );
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    #[cfg(unix)]
    fn init_creates_config_with_restricted_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join("colporteur-test-init-perms");
        let _ = std::fs::remove_dir_all(&dir);
        unsafe { std::env::set_var("XDG_CONFIG_HOME", &dir) };

        let result = Config::init();
        let path = result.expect("init should succeed");
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;

        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(mode, 0o600, "expected 0600, got {mode:o}");
    }
}
