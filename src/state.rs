use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eyre::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AppState {
    pub accounts: HashMap<String, AccountState>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct AccountState {
    pub uid_validity: u32,
    pub feeds: HashMap<String, FeedState>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct FeedState {
    pub last_uid: u32,
}

impl AppState {
    pub fn load(path: &Path) -> eyre::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(path)
            .wrap_err_with(|| format!("failed to read state file: {}", path.display()))?;
        serde_json::from_str(&contents)
            .wrap_err_with(|| format!("failed to parse state file: {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> eyre::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .wrap_err_with(|| format!("failed to create directories: {}", parent.display()))?;
        }
        let tmp = path.with_extension("tmp");
        let json = serde_json::to_string_pretty(self).wrap_err("failed to serialize state")?;
        std::fs::write(&tmp, &json)
            .wrap_err_with(|| format!("failed to write tmp file: {}", tmp.display()))?;
        std::fs::rename(&tmp, path)
            .wrap_err_with(|| format!("failed to rename tmp to: {}", path.display()))?;
        Ok(())
    }

    pub fn default_path() -> eyre::Result<PathBuf> {
        let base = dirs::data_local_dir()
            .ok_or_else(|| eyre::eyre!("could not determine local data directory"))?;
        Ok(base.join("colporteur").join("state.json"))
    }

    pub fn last_uid(&self, account: &str, sender: &str) -> u32 {
        self.accounts
            .get(account)
            .and_then(|a| a.feeds.get(sender))
            .map(|f| f.last_uid)
            .unwrap_or(0)
    }

    pub fn update_uid(&mut self, account: &str, sender: &str, uid: u32) {
        self.accounts
            .entry(account.to_owned())
            .or_default()
            .feeds
            .entry(sender.to_owned())
            .or_default()
            .last_uid = uid;
    }

    pub fn uid_validity(&self, account: &str) -> Option<u32> {
        self.accounts
            .get(account)
            .map(|a| a.uid_validity)
            .filter(|&v| v != 0)
    }

    pub fn set_uid_validity(&mut self, account: &str, validity: u32) {
        self.accounts
            .entry(account.to_owned())
            .or_default()
            .uid_validity = validity;
    }

    pub fn reset_account(&mut self, account: &str) {
        if let Some(acc) = self.accounts.get_mut(account) {
            acc.feeds.clear();
            acc.uid_validity = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("colporteur_test_{name}.json"))
    }

    #[test]
    fn default_state_has_zero_last_uid() {
        assert_eq!(AppState::default().last_uid("x", "y"), 0);
    }

    #[test]
    fn update_and_read_uid() {
        let mut state = AppState::default();
        state.update_uid("acct", "sender@example.com", 42);
        assert_eq!(state.last_uid("acct", "sender@example.com"), 42);
    }

    #[test]
    fn uid_validity_none_when_zero_or_missing() {
        let mut state = AppState::default();
        assert!(state.uid_validity("acct").is_none());
        state.set_uid_validity("acct", 0);
        assert!(state.uid_validity("acct").is_none());
    }

    #[test]
    fn set_and_read_uid_validity() {
        let mut state = AppState::default();
        state.set_uid_validity("acct", 12345);
        assert_eq!(state.uid_validity("acct"), Some(12345));
    }

    #[test]
    fn reset_account_clears_feeds() {
        let mut state = AppState::default();
        state.update_uid("acct", "a@x.com", 10);
        state.update_uid("acct", "b@x.com", 20);
        state.set_uid_validity("acct", 99);
        state.reset_account("acct");
        assert_eq!(state.last_uid("acct", "a@x.com"), 0);
        assert_eq!(state.last_uid("acct", "b@x.com"), 0);
        assert!(state.uid_validity("acct").is_none());
    }

    #[test]
    fn save_and_load_round_trip() {
        let path = temp_path("save_load");
        let _ = std::fs::remove_file(&path);

        let mut state = AppState::default();
        state.update_uid("acct", "feed@x.com", 7);
        state.set_uid_validity("acct", 555);
        state.save(&path).unwrap();

        let loaded = AppState::load(&path).unwrap();
        assert_eq!(loaded.last_uid("acct", "feed@x.com"), 7);
        assert_eq!(loaded.uid_validity("acct"), Some(555));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let path = temp_path("nonexistent_xyzzy");
        let _ = std::fs::remove_file(&path);
        let state = AppState::load(&path).unwrap();
        assert_eq!(state.last_uid("any", "any"), 0);
        assert!(state.accounts.is_empty());
    }
}
