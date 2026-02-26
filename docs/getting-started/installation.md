# Installation

## Pre-compiled binaries

Download a binary for your platform from the [latest release](https://github.com/sripwoud/colporteur/releases/latest). Binaries are available for Linux (x86_64, aarch64), macOS (Intel, Apple Silicon), and Windows (x64).

Extract it somewhere on your `PATH`, e.g. `~/.local/bin`.

## From crates.io

```bash
cargo install colporteur
```

## From source

```bash
git clone https://github.com/sripwoud/colporteur
cd colporteur
cargo build --release
```

The binary will be at `target/release/colporteur`.

## Verify

```bash
colporteur --version
```
