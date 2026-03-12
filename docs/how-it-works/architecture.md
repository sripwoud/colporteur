# Architecture

Colporteur is structured as a pipeline of focused modules, each handling one stage of the email-to-feed conversion.

## Module overview

```
main.rs          CLI dispatch, exit codes, logging setup
  ├── cli.rs         Clap-derived argument parsing
  ├── config.rs      TOML config loading and validation
  ├── state.rs       JSON state persistence (UID tracking)
  └── fetch.rs       Core orchestration
        ├── imap.rs      IMAP connection and email fetching
        ├── email.rs     Raw email parsing (mailparse)
        ├── feed.rs      Atom feed read/write (atom_syndication)
        └── sanitize.rs  HTML sanitization (ammonia)
```

## Data flow

```
Config (TOML)
  │
  ▼
Group feeds by account
  │
  ▼  (per account)
Resolve password (plain/command) ──► Connect IMAP (TLS)
  │
  ▼
Check UIDVALIDITY ──► Reset state if changed
  │
  ▼  (per feed, per sender)
IMAP SEARCH (FROM + UID range) ──► Get new UIDs
  │
  ▼  (per UID)
FETCH raw email ──► Parse headers + body ──► Sanitize HTML
  │
  ▼
Append to Atom feed (in memory)
  │
  ▼
Trim to max_entries
  │
  ▼
Write feed atomically (.tmp → rename)
  │
  ▼
Save state atomically (.tmp → rename)
```

## Error isolation

Errors are isolated at each boundary to prevent one failure from blocking the entire run:

- **Account level** — if connection or password resolution fails, all feeds for that account fail but other accounts continue
- **Feed level** — each feed is processed independently within an account
- **Email level** — if a single email fails to parse, it's skipped and processing continues

## Key abstractions

### `EmailSource` trait

The IMAP client implements the `EmailSource` trait, which is the only interface the fetch pipeline uses:

```rust
pub trait EmailSource {
    fn uid_validity(&mut self, mailbox: &str) -> Result<u32>;
    fn search_from_since_uid(&mut self, sender: &str, last_uid: u32) -> Result<Vec<u32>>;
    fn fetch_email(&mut self, uid: u32) -> Result<FetchedEmail>;
}
```

This allows the entire pipeline to be tested with a mock source, without any network access.

### Atomic writes

Both feed files and state files use the write-to-temporary-then-rename pattern. This prevents partial writes from corrupting data if the process is interrupted.

### Write order resilience

The pipeline writes in this order: **feed file → state file**. If a crash occurs after writing the feed but before updating state, the next run will re-fetch and re-process the same emails — which is safe because entries are deduplicated by `Message-ID`.
