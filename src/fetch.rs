use std::collections::HashMap;
use std::path::Path;

use eyre::Context;
use serde::Serialize;

use crate::config::{AccountConfig, Config, FeedConfig};
use crate::email;
use crate::feed::{append_entry, load_or_create, trim_entries, write_atomic};
use crate::imap::{EmailSource, ImapClient};
use crate::sanitize::{sanitize_html, text_to_html};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct FetchReport {
    pub feeds: Vec<FeedResult>,
    pub total_new: usize,
}

#[derive(Debug, Serialize)]
pub struct FeedResult {
    pub key: String,
    pub new_entries: usize,
    pub output: Option<String>,
    pub ok: bool,
    pub error: Option<String>,
}

pub struct AccountRunArgs<'a> {
    pub source: &'a mut dyn EmailSource,
    pub account_name: &'a str,
    pub account: &'a AccountConfig,
    pub feeds: &'a [(&'a str, &'a FeedConfig)],
    pub config: &'a Config,
    pub state: &'a mut AppState,
    pub state_path: &'a Path,
    pub dry_run: bool,
}

pub fn run(
    config: &Config,
    state: &mut AppState,
    state_path: &Path,
    dry_run: bool,
) -> eyre::Result<FetchReport> {
    let mut feeds_by_account: HashMap<&str, Vec<(&str, &FeedConfig)>> = HashMap::new();
    for (feed_key, feed_config) in &config.feeds {
        feeds_by_account
            .entry(feed_config.account.as_str())
            .or_default()
            .push((feed_key.as_str(), feed_config));
    }

    let mut all_results: Vec<FeedResult> = Vec::new();

    for (account_name, feeds) in &feeds_by_account {
        let account = match config.accounts.get(*account_name) {
            Some(a) => a,
            None => {
                log::error!("account '{account_name}' not found in config");
                for (feed_key, _) in feeds {
                    all_results.push(FeedResult {
                        key: (*feed_key).to_string(),
                        new_entries: 0,
                        output: None,
                        ok: false,
                        error: Some(format!("account '{account_name}' not found in config")),
                    });
                }
                continue;
            }
        };

        let password = account.resolve_password();

        let mut source = match ImapClient::connect(&account.server, &account.username, &password) {
            Ok(c) => c,
            Err(e) => {
                log::error!("account '{account_name}': connection failed: {e}");
                for (feed_key, _) in feeds {
                    all_results.push(FeedResult {
                        key: (*feed_key).to_string(),
                        new_entries: 0,
                        output: None,
                        ok: false,
                        error: Some(format!("connection failed: {e}")),
                    });
                }
                continue;
            }
        };

        let mut results = run_with_source(AccountRunArgs {
            source: &mut source,
            account_name,
            account,
            feeds,
            config,
            state,
            state_path,
            dry_run,
        });

        if let Err(e) = source.logout() {
            log::warn!("account '{account_name}': logout error: {e}");
        }

        all_results.append(&mut results);
    }

    let total_new = all_results.iter().map(|r| r.new_entries).sum();

    Ok(FetchReport {
        feeds: all_results,
        total_new,
    })
}

pub fn run_with_source(args: AccountRunArgs<'_>) -> Vec<FeedResult> {
    let AccountRunArgs {
        source,
        account_name,
        account,
        feeds,
        config,
        state,
        state_path,
        dry_run,
    } = args;

    let server_validity = match source.uid_validity(&account.mailbox) {
        Ok(v) => v,
        Err(e) => {
            log::error!("account '{account_name}': failed to get UIDVALIDITY: {e}");
            return feeds
                .iter()
                .map(|(feed_key, _)| FeedResult {
                    key: (*feed_key).to_string(),
                    new_entries: 0,
                    output: None,
                    ok: false,
                    error: Some(format!("failed to get UIDVALIDITY: {e}")),
                })
                .collect();
        }
    };

    if let Some(stored_validity) = state.uid_validity(account_name)
        && stored_validity != server_validity
    {
        log::warn!(
            "account '{account_name}': UIDVALIDITY changed ({stored_validity} -> \
             {server_validity}), resetting state"
        );
        state.reset_account(account_name);
    }
    state.set_uid_validity(account_name, server_validity);

    let mut results = Vec::new();

    for (feed_key, feed_config) in feeds {
        let result = process_feed(ProcessFeedArgs {
            source,
            account_name,
            feed_key,
            feed_config,
            config,
            state,
            state_path,
            dry_run,
        });
        results.push(result);
    }

    results
}

struct ProcessFeedArgs<'a> {
    source: &'a mut dyn EmailSource,
    account_name: &'a str,
    feed_key: &'a str,
    feed_config: &'a FeedConfig,
    config: &'a Config,
    state: &'a mut AppState,
    state_path: &'a Path,
    dry_run: bool,
}

fn process_feed(args: ProcessFeedArgs<'_>) -> FeedResult {
    let ProcessFeedArgs {
        source,
        account_name,
        feed_key,
        feed_config,
        config,
        state,
        state_path,
        dry_run,
    } = args;

    let output_path = Path::new(&config.output_dir).join(format!("{feed_key}.xml"));

    let mut feed = match load_or_create(&output_path, &feed_config.title) {
        Ok(f) => f,
        Err(e) => {
            log::error!("feed '{feed_key}': failed to load or create feed file: {e}");
            return FeedResult {
                key: feed_key.to_string(),
                new_entries: 0,
                output: None,
                ok: false,
                error: Some(format!("failed to load or create feed file: {e}")),
            };
        }
    };

    let mut new_entries: usize = 0;

    for sender in &feed_config.senders {
        let last_uid = state.last_uid(account_name, sender);

        let uids = match source.search_from_since_uid(sender, last_uid) {
            Ok(u) => u,
            Err(e) => {
                log::error!("feed '{feed_key}', sender '{sender}': search failed: {e}");
                continue;
            }
        };

        let mut highest_uid = last_uid;

        for uid in uids {
            let fetched = match source.fetch_email(uid) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("feed '{feed_key}': failed to fetch UID {uid}: {e}");
                    continue;
                }
            };

            let email_content = match email::parse(&fetched.raw) {
                Ok(c) => c,
                Err(e) => {
                    log::error!("feed '{feed_key}': failed to parse email UID {uid}: {e}");
                    if uid > highest_uid {
                        highest_uid = uid;
                    }
                    continue;
                }
            };

            let sanitized = match &email_content.html {
                Some(html) => sanitize_html(html),
                None => {
                    let text = email_content.text.as_deref().unwrap_or("");
                    text_to_html(text)
                }
            };

            append_entry(&mut feed, &email_content, &sanitized);
            new_entries += 1;

            if uid > highest_uid {
                highest_uid = uid;
            }
        }

        if highest_uid > last_uid {
            state.update_uid(account_name, sender, highest_uid);
        }
    }

    let max = config.max_entries_for(feed_key);
    trim_entries(&mut feed, max);

    if !dry_run {
        if let Err(e) = write_atomic(&feed, &output_path) {
            log::error!("feed '{feed_key}': failed to write feed: {e}");
            return FeedResult {
                key: feed_key.to_string(),
                new_entries,
                output: None,
                ok: false,
                error: Some(format!("failed to write feed: {e}")),
            };
        }

        if let Err(e) = state.save(state_path) {
            log::error!("feed '{feed_key}': failed to save state: {e}");
            return FeedResult {
                key: feed_key.to_string(),
                new_entries,
                output: Some(output_path.to_string_lossy().into_owned()),
                ok: false,
                error: Some(format!("failed to save state: {e}")),
            };
        }
    }

    FeedResult {
        key: feed_key.to_string(),
        new_entries,
        output: if dry_run {
            None
        } else {
            Some(output_path.to_string_lossy().into_owned())
        },
        ok: true,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use super::*;
    use crate::imap::FetchedEmail;

    struct MockEmailSource {
        uid_validity_val: u32,
        emails: HashMap<u32, (String, Vec<u8>)>,
    }

    impl EmailSource for MockEmailSource {
        fn uid_validity(&mut self, _mailbox: &str) -> eyre::Result<u32> {
            Ok(self.uid_validity_val)
        }

        fn search_from_since_uid(&mut self, sender: &str, last_uid: u32) -> eyre::Result<Vec<u32>> {
            let mut uids: Vec<u32> = self
                .emails
                .iter()
                .filter(|&(uid, (s, _))| *uid > last_uid && s == sender)
                .map(|(&uid, _)| uid)
                .collect();
            uids.sort();
            Ok(uids)
        }

        fn fetch_email(&mut self, uid: u32) -> eyre::Result<FetchedEmail> {
            let (_, raw) = self
                .emails
                .get(&uid)
                .ok_or_else(|| eyre::eyre!("mock: UID {uid} not found"))?;
            Ok(FetchedEmail {
                uid,
                raw: raw.clone(),
            })
        }
    }

    fn make_emails(pairs: &[(u32, &str, &[u8])]) -> HashMap<u32, (String, Vec<u8>)> {
        pairs
            .iter()
            .map(|&(uid, sender, raw)| (uid, (sender.to_string(), raw.to_vec())))
            .collect()
    }

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "colporteur_fetch_test_{}_{}",
                name,
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("failed to create test dir");
            Self { path }
        }

        fn output_dir(&self) -> String {
            self.path.join("output").to_string_lossy().into_owned()
        }

        fn state_path(&self) -> PathBuf {
            self.path.join("state.json")
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn single_sender_config(output_dir: &str) -> Config {
        let mut accounts = HashMap::new();
        accounts.insert(
            "test".to_string(),
            AccountConfig {
                server: "localhost".to_string(),
                username: "user@test.com".to_string(),
                password: "unused".to_string(),
                mailbox: "INBOX".to_string(),
            },
        );
        let mut feeds = HashMap::new();
        feeds.insert(
            "ideabrowser".to_string(),
            FeedConfig {
                title: "Ideabrowser Daily".to_string(),
                account: "test".to_string(),
                senders: vec!["notifications@mail.ideabrowser.com".to_string()],
                max_entries: None,
            },
        );
        Config {
            output_dir: output_dir.to_string(),
            max_entries: 50,
            accounts,
            feeds,
        }
    }

    #[test]
    fn full_pipeline_with_mock() {
        let dir = TestDir::new("full_pipeline");
        let config = single_sender_config(&dir.output_dir());
        let state_path = dir.state_path();

        let mut source = MockEmailSource {
            uid_validity_val: 1,
            emails: make_emails(&[(
                100,
                "notifications@mail.ideabrowser.com",
                include_bytes!("../tests/fixtures/sample1.eml"),
            )]),
        };
        let mut state = AppState::default();

        let feeds: Vec<(&str, &FeedConfig)> =
            config.feeds.iter().map(|(k, v)| (k.as_str(), v)).collect();

        let account = config.accounts.get("test").unwrap();

        let results = run_with_source(AccountRunArgs {
            source: &mut source,
            account_name: "test",
            account,
            feeds: &feeds,
            config: &config,
            state: &mut state,
            state_path: &state_path,
            dry_run: false,
        });

        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert!(result.ok, "expected ok=true, got error: {:?}", result.error);
        assert_eq!(result.new_entries, 1);
        assert_eq!(result.key, "ideabrowser");

        let feed_path = std::path::Path::new(&dir.output_dir()).join("ideabrowser.xml");
        assert!(feed_path.exists(), "feed XML file should exist");

        let xml = std::fs::read_to_string(&feed_path).unwrap();
        assert!(
            xml.contains("Idea of the Day"),
            "expected 'Idea of the Day' in feed XML"
        );

        assert_eq!(
            state.last_uid("test", "notifications@mail.ideabrowser.com"),
            100
        );
    }

    #[test]
    fn dry_run_does_not_write_files() {
        let dir = TestDir::new("dry_run");
        let config = single_sender_config(&dir.output_dir());
        let state_path = dir.state_path();

        let mut source = MockEmailSource {
            uid_validity_val: 1,
            emails: make_emails(&[(
                100,
                "notifications@mail.ideabrowser.com",
                include_bytes!("../tests/fixtures/sample1.eml"),
            )]),
        };
        let mut state = AppState::default();

        let feeds: Vec<(&str, &FeedConfig)> =
            config.feeds.iter().map(|(k, v)| (k.as_str(), v)).collect();

        let account = config.accounts.get("test").unwrap();

        let results = run_with_source(AccountRunArgs {
            source: &mut source,
            account_name: "test",
            account,
            feeds: &feeds,
            config: &config,
            state: &mut state,
            state_path: &state_path,
            dry_run: true,
        });

        assert_eq!(results[0].new_entries, 1);

        let feed_path = std::path::Path::new(&dir.output_dir()).join("ideabrowser.xml");
        assert!(
            !feed_path.exists(),
            "feed XML file must not exist in dry-run"
        );

        let loaded = AppState::load(&state_path).unwrap();
        assert_eq!(
            loaded.last_uid("test", "notifications@mail.ideabrowser.com"),
            0,
            "state must not be persisted in dry-run"
        );
    }

    #[test]
    fn skips_already_processed_uids() {
        let dir = TestDir::new("skip_processed");
        let config = single_sender_config(&dir.output_dir());
        let state_path = dir.state_path();

        let mut source = MockEmailSource {
            uid_validity_val: 1,
            emails: make_emails(&[(
                100,
                "notifications@mail.ideabrowser.com",
                include_bytes!("../tests/fixtures/sample1.eml"),
            )]),
        };
        let mut state = AppState::default();
        state.update_uid("test", "notifications@mail.ideabrowser.com", 100);
        state.set_uid_validity("test", 1);

        let feeds: Vec<(&str, &FeedConfig)> =
            config.feeds.iter().map(|(k, v)| (k.as_str(), v)).collect();

        let account = config.accounts.get("test").unwrap();

        let results = run_with_source(AccountRunArgs {
            source: &mut source,
            account_name: "test",
            account,
            feeds: &feeds,
            config: &config,
            state: &mut state,
            state_path: &state_path,
            dry_run: false,
        });

        assert_eq!(results[0].new_entries, 0);
    }

    #[test]
    fn multiple_senders_in_one_feed() {
        let dir = TestDir::new("multi_sender");
        let state_path = dir.state_path();

        let mut accounts = HashMap::new();
        accounts.insert(
            "test".to_string(),
            AccountConfig {
                server: "localhost".to_string(),
                username: "user@test.com".to_string(),
                password: "unused".to_string(),
                mailbox: "INBOX".to_string(),
            },
        );
        let mut feeds_map = HashMap::new();
        feeds_map.insert(
            "multi".to_string(),
            FeedConfig {
                title: "Multi Sender Feed".to_string(),
                account: "test".to_string(),
                senders: vec![
                    "notifications@mail.ideabrowser.com".to_string(),
                    "noreply@gesundheitsportal-privat.de".to_string(),
                ],
                max_entries: None,
            },
        );
        let config = Config {
            output_dir: dir.output_dir(),
            max_entries: 50,
            accounts,
            feeds: feeds_map,
        };

        let mut source = MockEmailSource {
            uid_validity_val: 1,
            emails: make_emails(&[
                (
                    101,
                    "notifications@mail.ideabrowser.com",
                    include_bytes!("../tests/fixtures/sample1.eml"),
                ),
                (
                    102,
                    "noreply@gesundheitsportal-privat.de",
                    include_bytes!("../tests/fixtures/sample2.eml"),
                ),
            ]),
        };
        let mut state = AppState::default();

        let feeds: Vec<(&str, &FeedConfig)> =
            config.feeds.iter().map(|(k, v)| (k.as_str(), v)).collect();

        let account = config.accounts.get("test").unwrap();

        let results = run_with_source(AccountRunArgs {
            source: &mut source,
            account_name: "test",
            account,
            feeds: &feeds,
            config: &config,
            state: &mut state,
            state_path: &state_path,
            dry_run: false,
        });

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].new_entries, 2);

        assert_eq!(
            state.last_uid("test", "notifications@mail.ideabrowser.com"),
            101
        );
        assert_eq!(
            state.last_uid("test", "noreply@gesundheitsportal-privat.de"),
            102
        );
    }
}

pub fn test_connections(
    config: &Config,
    account_filter: Option<&str>,
) -> Vec<(String, eyre::Result<()>)> {
    config
        .accounts
        .iter()
        .filter(|(name, _)| account_filter.is_none_or(|f| f == name.as_str()))
        .map(|(name, account)| {
            let password = account.resolve_password();
            let result = ImapClient::test_connection(&account.server, &account.username, &password)
                .wrap_err_with(|| format!("connection test failed for '{name}'"));
            (name.clone(), result)
        })
        .collect()
}
