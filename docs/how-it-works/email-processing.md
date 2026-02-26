# Email Processing

Colporteur processes emails in two stages: parsing the raw MIME message, then sanitizing the HTML body for safe feed consumption.

## Parsing

Raw emails are parsed using the `mailparse` crate. Colporteur extracts:

| Field       | Source                          | Fallback                              |
| ----------- | ------------------------------- | ------------------------------------- |
| Subject     | `Subject` header                | `"(no subject)"`                      |
| From        | `From` header                   | `"unknown"`                           |
| Date        | `Date` header → `DateTime<Utc>` | Current time                          |
| Message-ID  | `Message-ID` header             | Generated hash from from+date+subject |
| Body (HTML) | `text/html` part                | —                                     |
| Body (text) | `text/plain` part               | —                                     |

For multipart messages, the parser recursively walks all MIME parts and collects the first `text/html` and `text/plain` bodies found. HTML is preferred over plain text for feed content.

If only a plain text body is available, it's converted to HTML by escaping entities and wrapping paragraphs in `<p>` tags.

## Sanitization

HTML sanitization uses the `ammonia` crate with an allowlist approach — only known-safe tags and attributes are kept. Everything else is stripped.

### Allowed tags

`p`, `a`, `img`, `ul`, `ol`, `li`, `h1`–`h6`, `br`, `hr`, `strong`, `em`, `b`, `i`, `blockquote`, `pre`, `code`, `table`, `thead`, `tbody`, `tr`, `td`, `th`, `div`, `span`, `sup`, `sub`

### What gets stripped

- `<script>` and `<style>` blocks (content and tags)
- Event handler attributes (`onclick`, `onload`, etc.)
- Tracking pixels — images with both `width="1"` and `height="1"`
- Any tags not in the allowlist

### Tracking pixel detection

Newsletter emails commonly embed 1x1 invisible images for open tracking. Colporteur detects these by checking for `width="1"` and `height="1"` attributes (supporting both single and double quotes) and removes the entire `<img>` tag.
