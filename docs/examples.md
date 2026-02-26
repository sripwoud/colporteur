# Examples

## Single account, multiple feeds

```toml
output_dir = "/srv/feeds"

[accounts.mail]
server = "mail.mxroute.com"
username = "newsletters@domain.com"
password_env = "IMAP_PASSWORD"

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
output_dir = "/srv/feeds"

[accounts.mxroute]
server = "mail.mxroute.com"
username = "news@domain.com"
password_env = "IMAP_MXROUTE_PASSWORD"

[accounts.gmail]
server = "imap.gmail.com"
username = "user@gmail.com"
password_env = "IMAP_GMAIL_PASSWORD"

[feeds.ideabrowser]
title = "Ideabrowser Daily"
account = "mxroute"
senders = ["notifications@mail.ideabrowser.com"]

[feeds.google-updates]
title = "Google Updates"
account = "gmail"
senders = ["no-reply@accounts.google.com"]
```

## Cron job

Run every 15 minutes, quietly (only errors logged):

```bash
*/15 * * * * IMAP_PASSWORD="$(pass show email/newsletters)" colporteur fetch -q
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
Environment=IMAP_PASSWORD=your-password
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
