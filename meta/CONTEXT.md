# Colporteur

Convert email newsletters into Atom feeds. Connects to IMAP mailboxes, fetches messages from configured senders, sanitizes the HTML, and writes Atom XML files.

## Language

### Email side

**Account**:
A configured IMAP mailbox the user has authorized colporteur to read from.
_Avoid_: mailbox (overloaded with the IMAP folder concept), inbox.

**Sender**:
An email address colporteur filters messages by within an Account's mailbox.
_Avoid_: from-address, source, feed source.

**UIDVALIDITY**:
The IMAP server's identifier (RFC 3501) for the current incarnation of a mailbox's UID space. When it changes, all previously-recorded UIDs are meaningless. In colporteur, a mismatch resets every Sender Cursor for the affected Account.

**Account Session**:
A short-lived authenticated IMAP connection scoped to one Account, opened via `AccountSession::open(name, &AccountConfig)` in `src/imap.rs`. Owns the `ImapClient` and logs out on drop (warn-only on failure). Opening fails with an `AccountOpenError` (password-resolution vs. connection variants).
_Avoid_: imap session (the RFC term — Account Session is colporteur's wrapper around it), session (ambiguous; reserved for this concept).

### Feed side

**Feed**:
A single Atom XML file colporteur writes. One Feed aggregates messages from one or more Senders.
_Avoid_: channel, output.

**Feed Key**:
The user-facing identifier for a Feed (e.g. `tech-newsletters`); also the on-disk filename stem.

**Feed Body**:
The sanitized, ready-to-embed HTML for a single Atom `<entry><content>`. Produced by `email::parse` from the raw email's HTML part (sanitized via `sanitize::sanitize_html`) or, when no HTML part exists, from the text part (converted via `sanitize::text_to_html`). Stored on `EmailContent.feed_html`.
_Avoid_: html, body, sanitized html.

### State side

**Sender Cursor**:
The "what's-been-fetched" bookmark for a `(Account, Sender)` pair. Holds the highest UID observed and is stored in `state.json`. Minted via `AppState::cursor(...)` in `src/state.rs`; idempotently resets on UIDVALIDITY mismatch at mint time.
_Avoid_: feed cursor, sender state (the latter is the persisted struct, the former is the live API).

**SenderState**:
The persisted form of a Sender Cursor — currently `{ last_uid: u32 }` — keyed by Sender within an Account in `state.json`.

## Relationships

- A **Feed** aggregates one or more **Senders**, each living within an **Account**.
- A **Sender Cursor** is keyed by `(Account, Sender)` — never by Feed Key.
- A change in **UIDVALIDITY** for an Account invalidates every **Sender Cursor** under that Account.
- Each parsed email yields exactly one **Feed Body**; `EmailContent` is constructed only via `email::parse`, which guarantees `feed_html` is non-empty.

## Example dialogue

> **Dev:** "If a Feed pulls from three Senders, do they share a Cursor?"
> **Maintainer:** "No — each `(Account, Sender)` has its own Sender Cursor. The Feed is just a write-side aggregation; the read-side bookmark lives one level deeper."

> **Dev:** "What happens when UIDVALIDITY changes mid-run?"
> **Maintainer:** "The next `cursor()` mint for that Account compares the server's value against the stored one, logs a warn, and clears every Sender Cursor under that Account before returning. Subsequent mints in the same run see the new value as 'stored' and no-op."

## Flagged ambiguities

- The on-disk schema once used `feeds: HashMap<String, FeedState>` keyed by Sender email. The name "feeds" misled readers into thinking the cursor was per-Feed. Resolved: schema renamed `senders: HashMap<String, SenderState>` (state.json `version: 2`); v1 files are silently discarded with a warn on load.
