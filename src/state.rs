use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eyre::Context;
use serde::{Deserialize, Serialize};

use crate::fs_atomic;

#[derive(Debug, Default)]
pub struct AppState {
    accounts: HashMap<String, AccountState>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct AccountState {
    uid_validity: u32,
    senders: HashMap<String, SenderState>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct SenderState {
    pub last_uid: u32,
}

/// A cursor for a specific `(account, sender)` pair.
///
/// Borrows `AppState` mutably and exposes eagerly-written access to the
/// persisted high-water mark for that sender.
pub struct SenderCursor<'a> {
    state: &'a mut SenderState,
}

impl SenderCursor<'_> {
    /// Returns the last seen UID for this sender — pass this to IMAP `UID SEARCH`.
    pub fn since_uid(&self) -> u32 {
        self.state.last_uid
    }

    /// Advances the persisted high-water mark if `uid` is greater than the current one.
    ///
    /// Call this *before* any potentially-failing parse/sanitize step so that
    /// attempted-but-failed UIDs still advance the cursor.
    pub fn observed(&mut self, uid: u32) {
        if uid > self.state.last_uid {
            self.state.last_uid = uid;
        }
    }
}

/// Versioned wrapper used only for serialization (save).
#[derive(Serialize)]
struct AppStateFile<'a> {
    version: u32,
    accounts: &'a HashMap<String, AccountState>,
}

const STATE_VERSION: u32 = 2;

/// Versioned wrapper used only for deserialization (load).
#[derive(Deserialize)]
struct AppStateFileLoad {
    #[serde(default)]
    accounts: HashMap<String, AccountState>,
}

impl AppState {
    pub fn load(path: &Path) -> eyre::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(path)
            .wrap_err_with(|| format!("failed to read state file: {}", path.display()))?;
        let raw: serde_json::Value = serde_json::from_str(&contents)
            .wrap_err_with(|| format!("failed to parse state file: {}", path.display()))?;
        let version = raw
            .get("version")
            .and_then(|v| v.as_u64())
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(0);
        if version != STATE_VERSION {
            log::warn!(
                "state file '{}' has unsupported version {} (expected {}), discarding",
                path.display(),
                version,
                STATE_VERSION
            );
            return Ok(Self::default());
        }
        let file: AppStateFileLoad = serde_json::from_value(raw)
            .wrap_err_with(|| format!("failed to deserialize state file: {}", path.display()))?;
        Ok(Self { accounts: file.accounts })
    }

    pub fn save(&self, path: &Path) -> eyre::Result<()> {
        let file = AppStateFile { version: STATE_VERSION, accounts: &self.accounts };
        let json = serde_json::to_string_pretty(&file).wrap_err("failed to serialize state")?;
        fs_atomic::write_atomic(path, json.as_bytes())
    }

    pub fn default_path() -> eyre::Result<PathBuf> {
        let base = dirs::data_local_dir()
            .ok_or_else(|| eyre::eyre!("could not determine local data directory"))?;
        Ok(base.join("colporteur").join("state.json"))
    }

    /// Returns the persisted last-seen UID for `(account, sender)` without
    /// touching UIDVALIDITY — intended for read-only display (e.g. `list`).
    pub fn last_uid(&self, account: &str, sender: &str) -> u32 {
        self.accounts
            .get(account)
            .and_then(|a| a.senders.get(sender))
            .map(|s| s.last_uid)
            .unwrap_or(0)
    }

    /// Mints a per-`(account, sender)` cursor.
    ///
    /// On UIDVALIDITY mismatch the account's senders are cleared and the new
    /// validity is stored (idempotent: repeat mints with the same validity are
    /// no-ops).
    pub fn cursor<'a>(
        &'a mut self,
        account: &str,
        sender: &str,
        server_validity: u32,
    ) -> SenderCursor<'a> {
        let acc = self.accounts.entry(account.to_owned()).or_default();
        let stored_validity = acc.uid_validity;
        if stored_validity != 0 && stored_validity != server_validity {
            log::warn!(
                "account '{account}': UIDVALIDITY changed ({stored_validity} -> \
                 {server_validity}), resetting state"
            );
            acc.senders.clear();
        }
        acc.uid_validity = server_validity;
        let state = acc.senders.entry(sender.to_owned()).or_default();
        SenderCursor { state }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("colporteur_test_{name}.json"))
    }

    #[test]
    fn cursor_new_account_since_uid_is_zero() {
        let mut state = AppState::default();
        let cursor = state.cursor("acct", "sender@example.com", 1);
        assert_eq!(cursor.since_uid(), 0);
    }

    #[test]
    fn cursor_matching_validity_does_not_reset() {
        let mut state = AppState::default();
        {
            let mut c = state.cursor("acct", "a@x.com", 42);
            c.observed(10);
        }
        // Remint with the same validity — data must survive
        let c = state.cursor("acct", "a@x.com", 42);
        assert_eq!(c.since_uid(), 10);
    }

    #[test]
    fn cursor_mismatched_validity_resets_senders() {
        let mut state = AppState::default();
        {
            let mut c = state.cursor("acct", "a@x.com", 1);
            c.observed(10);
        }
        {
            let mut c = state.cursor("acct", "b@x.com", 1);
            c.observed(20);
        }
        // Mismatch: new validity 2
        {
            let c = state.cursor("acct", "a@x.com", 2);
            assert_eq!(c.since_uid(), 0, "sender a should be reset");
        }
        {
            let c = state.cursor("acct", "b@x.com", 2);
            assert_eq!(c.since_uid(), 0, "sender b should be reset");
        }
    }

    #[test]
    fn cursor_repeat_mint_same_validity_is_noop() {
        let mut state = AppState::default();
        // First mint triggers validity change from 0→1 (not a "mismatch")
        {
            let mut c = state.cursor("acct", "a@x.com", 1);
            c.observed(5);
        }
        // Mismatch: new validity 2 → clears senders
        {
            let _ = state.cursor("acct", "a@x.com", 2);
        }
        // Repeat with same new validity 2 — must not clear again
        {
            let mut c = state.cursor("acct", "a@x.com", 2);
            c.observed(99);
        }
        {
            let c = state.cursor("acct", "a@x.com", 2);
            assert_eq!(c.since_uid(), 99);
        }
    }

    #[test]
    fn observed_advances_last_uid_eagerly() {
        let mut state = AppState::default();
        let mut cursor = state.cursor("acct", "sender@example.com", 1);
        cursor.observed(42);
        assert_eq!(cursor.since_uid(), 42);
    }

    #[test]
    fn observed_out_of_order_keeps_maximum() {
        let mut state = AppState::default();
        let mut cursor = state.cursor("acct", "sender@example.com", 1);
        cursor.observed(50);
        cursor.observed(30); // lower — must not regress
        cursor.observed(70);
        cursor.observed(60); // lower — must not regress
        assert_eq!(cursor.since_uid(), 70);
    }

    #[test]
    fn observed_persists_through_new_cursor() {
        let mut state = AppState::default();
        {
            let mut c = state.cursor("acct", "sender@example.com", 1);
            c.observed(55);
        }
        // Drop cursor and re-mint — persisted value must survive
        let c = state.cursor("acct", "sender@example.com", 1);
        assert_eq!(c.since_uid(), 55);
    }

    #[test]
    fn save_and_load_round_trip() {
        let path = temp_path("save_load_v2");
        let _ = std::fs::remove_file(&path);

        let mut state = AppState::default();
        {
            let mut c = state.cursor("acct", "feed@x.com", 555);
            c.observed(7);
        }
        state.save(&path).unwrap();

        let mut loaded = AppState::load(&path).unwrap();
        let c = loaded.cursor("acct", "feed@x.com", 555);
        assert_eq!(c.since_uid(), 7);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let path = temp_path("nonexistent_xyzzy_v2");
        let _ = std::fs::remove_file(&path);
        let state = AppState::load(&path).unwrap();
        assert!(state.accounts.is_empty());
    }

    #[test]
    fn load_v1_file_discards_and_returns_default() {
        let path = temp_path("v1_state");
        let _ = std::fs::remove_file(&path);
        // Write a v1-style file (missing "version" field)
        std::fs::write(
            &path,
            r#"{"accounts":{"acct":{"uid_validity":1,"feeds":{"s@x.com":{"last_uid":99}}}}}"#,
        )
        .unwrap();
        let state = AppState::load(&path).unwrap();
        assert!(
            state.accounts.is_empty(),
            "v1 file should be discarded and return default"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn saved_state_is_v2_schema() {
        let path = temp_path("schema_v2");
        let _ = std::fs::remove_file(&path);

        let mut state = AppState::default();
        {
            let mut c = state.cursor("myaccount", "s@x.com", 7);
            c.observed(3);
        }
        state.save(&path).unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(v["version"], STATE_VERSION);
        assert_eq!(v["accounts"]["myaccount"]["uid_validity"], 7);
        assert_eq!(v["accounts"]["myaccount"]["senders"]["s@x.com"]["last_uid"], 3);

        let _ = std::fs::remove_file(&path);
    }
}
