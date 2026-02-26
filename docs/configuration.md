# Configuration

Colporteur reads its config from `~/.config/colporteur/config.toml`.

## Full example

```toml
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

[feeds.mixed]
title = "Health & Parenting"
account = "gmail"
senders = ["noreply@health.com", "tips@parenting.com"]
max_entries = 10
```

## Global settings

| Key           | Type    | Default    | Description                                          |
| ------------- | ------- | ---------- | ---------------------------------------------------- |
| `output_dir`  | string  | _required_ | Directory where Atom XML feed files are written      |
| `max_entries` | integer | `50`       | Maximum entries kept per feed (oldest trimmed first) |

## Account settings

Defined under `[accounts.<name>]`.

| Key            | Type   | Default    | Description                                                |
| -------------- | ------ | ---------- | ---------------------------------------------------------- |
| `server`       | string | _required_ | IMAP server hostname                                       |
| `username`     | string | _required_ | IMAP username                                              |
| `password_env` | string | _required_ | Name of the environment variable holding the IMAP password |
| `mailbox`      | string | `"INBOX"`  | IMAP mailbox to search                                     |

## Feed settings

Defined under `[feeds.<name>]`. The `<name>` becomes the output filename (`<name>.xml`).

| Key           | Type     | Default              | Description                                                   |
| ------------- | -------- | -------------------- | ------------------------------------------------------------- |
| `title`       | string   | _required_           | Title of the Atom feed                                        |
| `account`     | string   | _required_           | Key of the account to use (must match an `[accounts.<name>]`) |
| `senders`     | string[] | _required_           | Email addresses to search for in the mailbox                  |
| `max_entries` | integer  | global `max_entries` | Per-feed override for max entries                             |

## Passwords

Passwords are never stored in the config file. Instead, `password_env` specifies the name of an environment variable that holds the password. This allows integration with secret managers, `.env` files, or shell exports:

```bash
export IMAP_MXROUTE_PASSWORD="$(pass show email/mxroute)"
```

## State

Colporteur persists sync state at `~/.local/share/colporteur/state.json`. This tracks:

- **UIDVALIDITY** per account — detects mailbox resets
- **Last seen UID** per sender — enables incremental fetching

If UIDVALIDITY changes (mailbox was rebuilt), the state for that account is reset and all emails are re-fetched.

## Validation

Config is validated on load. Colporteur exits with code `3` if:

- `output_dir` is empty
- No accounts are defined
- No feeds are defined
- A feed references a non-existent account
