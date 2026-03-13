# CLI Reference

```
colporteur [OPTIONS] <COMMAND>
```

## Global options

| Flag        | Short | Description                                     |
| ----------- | ----- | ----------------------------------------------- |
| `--json`    |       | Output in JSON format (to stdout)               |
| `--quiet`   | `-q`  | Suppress all output except errors               |
| `--verbose` | `-v`  | Increase verbosity (`-v` = info, `-vv` = debug) |
| `--version` |       | Print version                                   |
| `--help`    | `-h`  | Print help                                      |

## Commands

### `fetch`

Fetch new newsletter emails and update Atom feed files.

```bash
colporteur fetch [--feed FEED] [--dry-run]
```

| Flag            | Description                                                        |
| --------------- | ------------------------------------------------------------------ |
| `--feed <FEED>` | Process only this feed (must match a key in config)                |
| `--dry-run`     | Parse and process emails but don't write any files or update state |

**Exit codes:**

| Code | Meaning                             |
| ---- | ----------------------------------- |
| `0`  | All feeds processed successfully    |
| `1`  | All feeds failed                    |
| `4`  | Some feeds failed (partial failure) |

**Example:**

```bash
colporteur fetch
colporteur fetch --feed ideabrowser
colporteur fetch --dry-run --json
```

### `test`

Test IMAP connection(s).

```bash
colporteur test [--account ACCOUNT]
```

| Flag                  | Description                                         |
| --------------------- | --------------------------------------------------- |
| `--account <ACCOUNT>` | Test only this account (must match a key in config) |

**Exit codes:**

| Code | Meaning                        |
| ---- | ------------------------------ |
| `0`  | All connections succeeded      |
| `5`  | One or more connections failed |

**Example:**

```bash
colporteur test
colporteur test --account gmail --json
```

### `list`

List configured feeds and their sync state.

```bash
colporteur list
```

**Example output:**

```
FEED                 ACCOUNT      SENDERS                              LAST UID
ideabrowser          mxroute      notifications@mail.ideabrowser.com   142
mixed                gmail        noreply@health.com, tips@parent...   87
```

### `export-opml`

Export configured feeds as an OPML 2.0 file for importing into RSS readers.

```bash
colporteur export-opml --base-url URL [-o FILE]
```

| Flag               | Short | Description                                       |
| ------------------ | ----- | ------------------------------------------------- |
| `--base-url <URL>` |       | Base URL for feed links (required; no `?` or `#`) |
| `--output <FILE>`  | `-o`  | Output file path (default: stdout)                |

**Example:**

```bash
colporteur export-opml --base-url https://feeds.example.com
colporteur export-opml --base-url https://feeds.example.com -o feeds.opml
colporteur export-opml --base-url https://feeds.example.com -o feeds.opml -q
```

## Logging

Verbosity is controlled by `-v` flags or the `RUST_LOG` environment variable:

| Level | Flag      | What you see                    |
| ----- | --------- | ------------------------------- |
| Error | (default) | Only errors                     |
| Warn  | (default) | Errors + warnings               |
| Info  | `-v`      | Progress messages               |
| Debug | `-vv`     | Detailed IMAP/parse diagnostics |

If `RUST_LOG` is set, it takes precedence over `-v`/`-q` flags.
