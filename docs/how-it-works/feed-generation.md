# Feed Generation

Colporteur produces standard [Atom 1.0](https://www.rfc-editor.org/rfc/rfc4287) feeds using the `atom_syndication` crate.

## Feed structure

Each configured feed maps to one XML file at `<output_dir>/<feed_key>.xml`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Ideabrowser Daily</title>
  <updated>2026-02-24T08:00:00Z</updated>
  <entry>
    <id>abc123@mail.ideabrowser.com</id>
    <title>Idea of the Day: Modular Furniture</title>
    <author><name>notifications@mail.ideabrowser.com</name></author>
    <updated>2026-02-24T08:00:00Z</updated>
    <content type="html">...</content>
  </entry>
</feed>
```

## Entry IDs

Each entry needs a globally unique ID. Colporteur uses:

1. The `Message-ID` header (stripped of angle brackets) — preferred
2. A fallback `urn:colporteur:<hash>` derived from `from + date + subject` — when no Message-ID is present

This ensures entries are stable across re-runs and can be deduplicated by feed readers.

## Entry ordering

New entries are prepended (newest first). When the feed is loaded from an existing file, new entries are added at the top.

## Trimming

After adding new entries, the feed is trimmed to `max_entries` (default 50, configurable globally or per-feed). The oldest entries are removed.

## Atomic writes

Feed files are written using the write-to-temporary-then-rename pattern:

1. Write content to `<feed_key>.xml.tmp`
2. Rename `.tmp` to `.xml`

This ensures readers never see a partially-written file. If the process crashes mid-write, the temporary file is left behind and the previous feed remains intact.
