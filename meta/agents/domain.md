# Domain Docs

How the engineering skills should consume this repo's domain documentation when exploring the codebase.

## Before exploring, read these

- **`meta/CONTEXT.md`** — repo glossary and domain language (single-context repo).
- **`meta/adr/`** — read ADRs that touch the area you're about to work in.

If any of these files don't exist, **proceed silently**. Don't flag their absence; don't suggest creating them upfront. The producer skill (`/grill-with-docs`) creates them lazily when terms or decisions actually get resolved.

> Note: this repo uses `meta/` instead of the default `CONTEXT.md` at the root and `docs/adr/`. The reason: the existing `docs/` directory is a published docsify site, so internal-only documents must live elsewhere. Always look in `meta/`, not `docs/`, for ADRs and context.

## File structure

Single-context repo, custom paths:

```
/
├── meta/
│   ├── CONTEXT.md                 ← created lazily by /grill-with-docs
│   └── adr/
│       ├── 0001-*.md
│       └── 0002-*.md
├── docs/                          ← published docsify site (do NOT put ADRs here)
└── src/
```

## Use the glossary's vocabulary

When your output names a domain concept (in an issue title, a refactor proposal, a hypothesis, a test name), use the term as defined in `meta/CONTEXT.md`. Don't drift to synonyms the glossary explicitly avoids.

If the concept you need isn't in the glossary yet, that's a signal — either you're inventing language the project doesn't use (reconsider) or there's a real gap (note it for `/grill-with-docs`).

## Flag ADR conflicts

If your output contradicts an existing ADR, surface it explicitly rather than silently overriding:

> _Contradicts ADR-0007 (event-sourced orders) — but worth reopening because…_
