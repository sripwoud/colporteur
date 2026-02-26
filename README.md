<p align="center">
  <h1 align="center"><a href="https://colporteur.sripwoud.xyz">Colporteur</a></h1>
</p>
<p align="center">
  <a href="https://crates.io/crates/colporteur">
    <img src="https://img.shields.io/crates/v/colporteur" alt="Crates.io">
  </a>
</p>

> Convert email newsletters into Atom feeds

Colporteur connects to IMAP mailboxes, fetches emails from configured senders, sanitizes the HTML, and writes Atom XML feed files. Subscribe to your newsletters with any feed reader.

## Features

- **IMAP native** -- connects directly to any IMAP mailbox, no forwarding rules needed
- **HTML sanitization** -- strips tracking pixels, scripts, and styles while preserving content
- **Atom feeds** -- generates standard Atom XML consumable by any feed reader
- **Multi-account** -- pull newsletters from multiple IMAP accounts into separate feeds
- **Dry-run mode** -- preview what would be fetched before writing anything
- **JSON output** -- machine-readable output for scripting and automation

## Quick Start

Install colporteur:

```bash
cargo install colporteur
```

Generate a sample config:

```bash
colporteur init
```

Edit `~/.config/colporteur/config.toml` with your IMAP accounts and feeds, then:

```bash
export IMAP_PASSWORD="your-password"
colporteur test    # verify IMAP connection
colporteur fetch   # fetch and generate feeds
colporteur list    # show feed sync state
```

## Documentation

Full documentation available at [colporteur.sripwoud.xyz](https://colporteur.sripwoud.xyz):

- [Installation](https://colporteur.sripwoud.xyz/#/getting-started/installation) - Detailed setup guide
- [Quick Start](https://colporteur.sripwoud.xyz/#/getting-started/quick-start) - Step-by-step walkthrough
- [CLI Reference](https://colporteur.sripwoud.xyz/#/cli-reference) - All commands documented
- [Configuration](https://colporteur.sripwoud.xyz/#/configuration) - Config file format and options
- [How It Works](https://colporteur.sripwoud.xyz/#/how-it-works/email-processing) - Email processing pipeline

## Requirements

- Rust/Cargo for installation
- An IMAP-accessible email account with newsletters

## Develop

| Feature                                        | With                                                                                   | Configuration                                          |
| ---------------------------------------------- | -------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| Continuous Integration                         | [GitHub Workflow](https://docs.github.com/en/actions/using-workflows)                  | [.github/workflows](./.github/workflows)               |
| Conventional Commits                           | [convco](https://github.com/convco/convco)                                             | [.convco](./.convco)                                   |
| Conventional PR Titles                         | [action-semantic-pull-request](https://github.com/amannn/action-semantic-pull-request) | [semantic-pr.yml](./.github/workflows/semantic-pr.yml) |
| Documentation                                  | [docsify](https://docsify.js.org/)                                                     | [docs/](./docs)                                        |
| Formatting                                     | [dprint](https://dprint.dev/)                                                          | [.dprint.jsonc](./.dprint.jsonc)                       |
| Git Hooks                                      | [hk](https://hk.jdx.dev/)                                                              | [hk.pkl](./hk.pkl)                                     |
| Tasks Runner, Environment & Runtime Management | [mise](https://mise.dev/)                                                              | [mise.toml](./mise.toml)                               |

```bash
./setup       # install mise and setup repository
mise run      # run tasks interactively
```

## Community

- [Documentation](https://colporteur.sripwoud.xyz)
- [Report Issues](https://github.com/sripwoud/colporteur/issues)
