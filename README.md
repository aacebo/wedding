# Baicebo Wedding Planning

## Admin planning hub (`/admin`)

A hidden, authenticated area that aggregates wedding communications into todos,
a timeline, and deadline reminders. It is intentionally invisible: any request
from someone who is not signed in as an allowlisted user returns **404**.

### Environment variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `PORT` | no | `8080` | HTTP port. |
| `DATABASE_URL` | no (prod: yes) | local dev URL | Postgres connection string. |
| `ADMIN_ALLOWLIST` | yes (for `/admin`) | empty | Comma-separated emails allowed into `/admin`. |
| `SESSION_SECRET` | yes (prod) | insecure dev value | Secret used to sign the admin session cookie. Use a long random value (≥32 bytes). |
| `SESSION_COOKIE_SECURE` | no | `true` | Set to `false` for local HTTP dev so the session cookie is sent over `http://`. Leave `true` in production (HTTPS). |
| `ADMIN_DEV_LOGIN` | no | `false` | When `true`, enables the temporary `POST /admin/dev-login` endpoint to establish a session before Google SSO exists. **Never enable in production.** |
| `GOOGLE_CLIENT_ID` | for SSO | unset | OAuth client id from Google Cloud Console. |
| `GOOGLE_CLIENT_SECRET` | for SSO | unset | OAuth client secret. |
| `GOOGLE_REDIRECT_URI` | for SSO | unset | Must exactly match an authorized redirect URI, e.g. `https://baicebo.com/admin/auth/google/callback` (or `http://localhost:8080/admin/auth/google/callback` in dev). |
| `TOKEN_ENCRYPTION_KEY` | for SSO | unset | Secret used to derive the AES-256 key that encrypts stored Google tokens at rest. Use a long random value. |
| `OPENAI_API_KEY` | for extraction | unset | OpenAI API key. When unset, `/admin/extract` returns 404 and the LLM pipeline is disabled. |
| `OPENAI_MODEL` | no | `gpt-4o-mini` | OpenAI model used for extraction. |
| `LLM_MAX_BATCH` | no | `25` | Max sources processed per `/admin/extract` run (cost cap). |

Google SSO is enabled only when **all four** `GOOGLE_*` / `TOKEN_ENCRYPTION_KEY`
values are set; otherwise `/admin/auth/google/*` returns 404 and you fall back to
`ADMIN_DEV_LOGIN`. Configure the OAuth consent screen with read-only Gmail
(`gmail.readonly`) and Drive (`drive.readonly`) scopes, and add the redirect URI
above. Only allowlisted emails can complete the callback; anyone else is signed
out and gets a 404.

### Local development

```sh
# start Postgres (see docker-compose.yml) then:
ADMIN_ALLOWLIST="you@gmail.com,partner@gmail.com" \
ADMIN_DEV_LOGIN=true \
SESSION_COOKIE_SECURE=false \
cargo run -p api

# sign in (dev only), then visit http://localhost:8080/admin
curl -i -c cookies.txt -X POST http://localhost:8080/admin/dev-login \
  -H 'content-type: application/json' -d '{"email":"you@gmail.com"}'
curl -i -b cookies.txt http://localhost:8080/admin
```

To exercise real Google SSO locally, also set the four Google variables and start
the flow at `http://localhost:8080/admin/auth/google/login`:

```sh
ADMIN_ALLOWLIST="you@gmail.com,partner@gmail.com" \
SESSION_COOKIE_SECURE=false \
GOOGLE_CLIENT_ID=... GOOGLE_CLIENT_SECRET=... \
GOOGLE_REDIRECT_URI=http://localhost:8080/admin/auth/google/callback \
TOKEN_ENCRYPTION_KEY="$(openssl rand -hex 32)" \
cargo run -p api
```

Real Google SSO replaces `dev-login`; see `backlogs/`.

### Ingesting Gmail &amp; Drive

Once an allowlisted Google account is linked, the admin hub can pull read-only
communications into a normalized `comm_sources` table:

- `POST /admin/sync` — runs Gmail + Drive ingestion for every linked account and
  returns per-account `new`/`updated`/`skipped` counts (failures are isolated per
  account/provider). The `/admin` dashboard exposes this as a **Sync now** button.
- `GET /admin/sync/status` — last sync time + cursor per account/provider and the
  total number of ingested sources.

Syncs are incremental: Gmail advances an epoch-seconds `after:` cursor and Drive
uses the changes-feed page token, both stored in `sync_state`. Re-running only
fetches deltas; the `UNIQUE (provider, external_id)` constraint deduplicates.
Google Docs are exported to plain text into `body_text`. Scheduled syncs are
Phase 07.

### Extracting todos, timeline &amp; deadlines

With `OPENAI_API_KEY` set, the admin hub distills unprocessed `comm_sources` into
structured planning items via OpenAI:

- `POST /admin/extract` — processes up to `LLM_MAX_BATCH` unprocessed sources,
  emitting todos, timeline events, and deadlines linked back to their source, and
  returns per-run counts plus running totals. 404s when `OPENAI_API_KEY` is unset.

Extraction is idempotent: re-running a source deletes only its **AI-created,
still-open** items before re-inserting, so human edits and completed/dismissed
items survive. Email/doc text is treated as **untrusted** — the system prompt
ignores instructions embedded in content (prompt-injection defense). Each call is
audited in `llm_runs` (model, token usage, estimated cost); on failure the source
is left unprocessed for the next run. A source is re-extracted automatically when
its content changes (an upsert resets `processed_at`).

### The planning dashboard

The `/admin` hub renders four server-side (Askama + htmx + Alpine) views behind
the `AdminSession` guard:

- **Dashboard** (`/admin`) — open-todo/event/deadline counts, the next upcoming
  deadlines, recent communications, and one-click **Sync now** / **Extract todos**
  buttons (each shown only when its integration is configured).
- **Inbox** (`/admin/inbox`) — aggregated `comm_sources`, filterable by provider
  (`?provider=gmail|drive`); each row links out to the original Gmail/Drive item.
- **Todos** (`/admin/todos`) — add/complete/dismiss/reopen/edit/delete inline via
  htmx. Low-confidence AI suggestions are flagged **Review**; each todo links back
  to its source.
- **Timeline** (`/admin/timeline`) — events and deadlines merged and sorted by
  date, with add/complete/delete controls.

Human edits are durable: editing an AI item flips its `created_by` to `human`, and
completing/dismissing changes its status — both of which the extraction re-run
skips (it only replaces AI-created, still-`open` items), so manual work is never
clobbered.

### Notifications & deadline reminders

Every admin page shows a **bell** in the nav with an unread count that polls every
60s (`GET /admin/notifications/count`). Opening it loads a panel
(`GET /admin/notifications`) listing reminders derived from open, dated deadlines
and todos:

- `deadline_overdue` — past due (shown first, in red).
- `deadline_soon` / `todo_due` — due within the next 14 days.

`generate_notifications()` (storage `NotificationStorage::generate`) is idempotent:
one notification per referenced item (`UNIQUE (ref_type, ref_id)`), whose `kind`
escalates in place as a deadline moves from soon to overdue, and which is pruned
once its item is completed, dismissed, or undated. It runs automatically at the end
of each `/admin/extract` pass and on the panel's **Refresh** button
(`POST /admin/notifications/refresh`). Reminders can be marked read
(`PATCH /admin/notifications/{id}?value=read`), dismissed (`value=dismissed`), or
cleared in bulk (`POST /admin/notifications/read-all`); read/dismissed state
survives regeneration. The `notifications` schema is channel-agnostic, so an
email/push sender can be added later without a migration.

### Scheduled background sync

The full pipeline — **ingest Gmail/Drive → LLM extraction → regenerate
notifications** — is a single reusable pass, `pipeline::run_once(ctx, trigger)`.
It can be driven three ways, all sharing one code path and one audit trail:

- **Scheduled (production):** a Render **cron** service (`wedding-sync`,
  `schedule: "*/30 * * * *"`) runs the one-shot CLI `/(/app/api) sync-once`, which
  builds the context, runs a single pass, prints a JSON report, and exits.
- **Manual:** the dashboard's **Run full sync** button posts to `POST /admin/pipeline`
  (shown only when Google or OpenAI is configured).
- **Granular:** the existing `POST /admin/sync` (ingest only) and `POST /admin/extract`
  (extract only) endpoints are unchanged.

**Overlap protection:** each pass takes a Postgres session-scoped **advisory lock**
(`pg_try_advisory_lock`). If a scheduled run and a manual trigger coincide, the
second is skipped (`{"skipped": true}`) rather than double-processing — safe even
if the web service scales to multiple instances. Every pass is recorded in the
`sync_runs` audit table (trigger, status, per-stage counts, error, timestamps),
and the dashboard's **System status** widget surfaces the latest run and any error.

Run a one-off pass locally:

```sh
DATABASE_URL=postgres://admin:admin@localhost:5432/wedding \
cargo run -p api -- sync-once
```

### Deployment (Render)

`render.yaml` provisions the Postgres database, the `wedding-api` web service, and
the `wedding-sync` cron service. Secrets are declared `sync: false` so Render
prompts for them at Blueprint creation (they are never committed); `SESSION_SECRET`
is generated. Set the same Google/OpenAI secrets on both services. Copy
`.env.example` to `.env` for local development.

**Google Cloud OAuth setup:** create an OAuth 2.0 Client (Web application), add the
authorized redirect URI `https://baicebo.com/admin/auth/google/callback` (and the
`http://localhost:8080/...` variant for dev), and request only the read-only
`gmail.readonly` and `drive.readonly` scopes plus `openid`/`email`. Put the two
allowlisted wedding emails on the OAuth consent screen's test users.

**Security notes:** Google scopes are read-only; tokens are encrypted at rest
(AES-256 derived from `TOKEN_ENCRYPTION_KEY`); the allowlist is enforced on every
`/admin/*` route (covered by an integration test) and unauthenticated requests get
404; email/doc content is treated as untrusted LLM input; and no tokens, PII, or
secrets are logged.
