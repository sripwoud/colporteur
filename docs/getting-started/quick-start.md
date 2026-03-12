# Quick Start

## 1. Create a config file

Colporteur reads its config from `~/.config/colporteur/config.toml`:

```toml
output_dir = "/var/lib/colporteur/feeds"

[accounts.mxroute]
server = "mail.mxroute.com"
username = "news@domain.com"
password = "your-imap-password"

[feeds.ideabrowser]
title = "Ideabrowser Daily"
account = "mxroute"
senders = ["notifications@mail.ideabrowser.com"]
```

## 2. Test the connection

```bash
colporteur test
```

```
testing accounts...
  mxroute      mail.mxroute.com               ok
```

## 3. Fetch newsletters

```bash
colporteur fetch
```

```
fetching feeds...
  ideabrowser          1 new  ->  /var/lib/colporteur/feeds/ideabrowser.xml
done. 1 entries written.
```

## 4. Subscribe in your feed reader

Point your feed reader at `/var/lib/colporteur/feeds/ideabrowser.xml`. Run `colporteur fetch` periodically (e.g. via cron) to keep it updated.

## Next steps

- See [Configuration](configuration.md) for the full config reference
- See [CLI Reference](cli-reference.md) for all commands and flags
- See [Examples](examples.md) for common setups
