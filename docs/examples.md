# Examples

## Single account, multiple feeds

```toml
output_dir = "/var/lib/colporteur/feeds"

[accounts.mail]
server = "mail.mxroute.com"
username = "newsletters@domain.com"
password = "secret" # or "!pass show email/newsletters" for a password manager

[feeds.tech]
title = "Tech Newsletters"
account = "mail"
senders = ["weekly@changelog.com", "digest@hackernewsletter.com"]

[feeds.health]
title = "Health Updates"
account = "mail"
senders = ["noreply@health-portal.com"]
max_entries = 20
```

## Multiple accounts

```toml
output_dir = "/var/lib/colporteur/feeds"

[accounts.mxroute]
server = "mail.mxroute.com"
username = "news@domain.com"
password = "secret" # or "!pass show email/mxroute" for a password manager

[accounts.gmail]
server = "imap.gmail.com"
username = "user@gmail.com"
password = "secret" # or "!op read 'op://Vault/Gmail/password'"

[feeds.ideabrowser]
title = "Ideabrowser Daily"
account = "mxroute"
senders = ["notifications@mail.ideabrowser.com"]

[feeds.google-updates]
title = "Google Updates"
account = "gmail"
senders = ["no-reply@accounts.google.com"]
```

## Feed entry links

```toml
output_dir = "/var/lib/colporteur/feeds"
base_url = "https://feeds.example.com"

[accounts.mail]
server = "mail.mxroute.com"
username = "newsletters@domain.com"
password = "secret"

[feeds.tech]
title = "Tech Newsletters"
account = "mail"
senders = ["weekly@changelog.com"]
# entry links → https://feeds.example.com/tech.xml

[feeds.substack-author]
title = "Author's Substack"
account = "mail"
senders = ["newsletter@substack.com"]
url = "https://author.substack.com/archive"
# entry links → https://author.substack.com/archive (overrides base_url)
```

Setting `base_url` automatically generates entry links. Use per-feed `url` to override with a custom link destination.

## Cron job

Run every 15 minutes, quietly (only errors logged):

```bash
*/15 * * * * colporteur fetch -q
```

## Fetch a single feed

```bash
colporteur fetch --feed ideabrowser
```

## Preview without writing

```bash
colporteur fetch --dry-run
colporteur fetch --dry-run --json | jq '.feeds[].new_entries'
```

## JSON output for scripting

```bash
colporteur list --json | jq '.[].feed'
colporteur test --json | jq '.[] | select(.ok == false)'
colporteur fetch --json | jq '.total_new'
```

## Systemd timer

`~/.config/systemd/user/colporteur.service`:

```ini
[Unit]
Description=Fetch newsletter feeds

[Service]
Type=oneshot
ExecStart=%h/.cargo/bin/colporteur fetch -q
```

`~/.config/systemd/user/colporteur.timer`:

```ini
[Unit]
Description=Fetch newsletter feeds every 15 minutes

[Timer]
OnCalendar=*:0/15
Persistent=true

[Install]
WantedBy=timers.target
```

```bash
systemctl --user enable --now colporteur.timer
```
