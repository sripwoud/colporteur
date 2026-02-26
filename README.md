# Colporteur

Convert email newsletters into Atom feeds.

Colporteur connects to IMAP mailboxes, fetches emails from configured senders, sanitizes the HTML, and writes Atom XML feed files. Subscribe to your newsletters with any feed reader.

## Usage

```bash
cargo install colporteur
```

Create `~/.config/colporteur/config.toml`:

```toml
output_dir = "/srv/feeds"

[accounts.mail]
server = "imap.example.com"
username = "newsletters@example.com"
password_env = "IMAP_PASSWORD"

[feeds.weekly-digest]
title = "Weekly Digest"
account = "mail"
senders = ["digest@newsletter.com"]
```

```bash
export IMAP_PASSWORD="your-password"
colporteur test    # verify connection
colporteur fetch   # fetch and generate feeds
colporteur list    # show feed sync state
```

Full documentation at [colporteur.sripwoud.xyz](https://colporteur.sripwoud.xyz).

## Develop

| Feature                                        | With                                                                                   | Configuration File                                     |
| ---------------------------------------------- | -------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| Continuous Integration                         | [GitHub Workflow](https://docs.github.com/en/actions/using-workflows)                  | [.github/workflows](./.github/workflows)               |
| Conventional Commits                           | [convco](https://github.com/convco/convco)                                             | [.convco](./.convco)                                   |
| Conventional PR Titles                         | [action-semantic-pull-request](https://github.com/amannn/action-semantic-pull-request) | [semantic-pr.yml](./.github/workflows/semantic-pr.yml) |
| Documentation                                  | [docsify](https://docsify.js.org/)                                                     | [docs/](./docs)                                        |
| Formatting                                     | [dprint](https://dprint.dev/)                                                          | [.dprint.jsonc](./.dprint.jsonc)                       |
| Git Hooks                                      | [hk](https://hk.jdx.dev/)                                                              | [hk.pkl](./hk.pkl)                                     |
| Tasks Runner, Environment & Runtime Management | [mise](https://mise.dev/)                                                              | [mise.toml](./mise.toml)                               |

I use [`mise`](https://mise.jdx.dev) to manage runtimes, manage environment variables, and run tasks.\
To install it and setup the repository:

```commandline
./setup
```

To run tasks interactively:

```commandline
mise run
```
