# Colporteur

> Convert email newsletters into Atom feeds.

Colporteur connects to your IMAP mailbox, fetches emails from configured newsletter senders, sanitizes the HTML, and writes Atom XML feed files to disk. Subscribe to your newsletters with any feed reader.

## Features

- **IMAP fetching** — connects to any IMAP server over TLS
- **Atom feed generation** — produces standard Atom XML consumable by any feed reader
- **HTML sanitization** — strips tracking pixels, scripts, and unsafe markup via allowlist
- **Incremental sync** — tracks last-seen UID per sender, only fetches new emails
- **UIDVALIDITY handling** — detects mailbox resets and re-syncs automatically
- **Multi-account** — configure multiple IMAP accounts and map feeds across them
- **Multi-sender feeds** — aggregate multiple senders into a single feed
- **Dry-run mode** — preview what would be fetched without writing anything
- **JSON output** — machine-readable output for scripting (`--json`)
- **Atomic writes** — feed files and state are written atomically (write-to-tmp then rename)

## Quick Start

```bash
cargo install colporteur
```

Create a config file at `~/.config/colporteur/config.toml`:

```toml
output_dir = "/srv/feeds"

[accounts.mail]
server = "imap.example.com"
username = "newsletters@example.com"
password = "your-password"

[feeds.weekly-digest]
title = "Weekly Digest"
account = "mail"
senders = ["digest@newsletter.com"]
```

Fetch:

```bash
colporteur fetch
```

The generated feed will be at `/srv/feeds/weekly-digest.xml`.

See [Installation](getting-started/installation.md) and [Quick Start](getting-started/quick-start.md) for detailed setup instructions.
