# Baicebo Wedding Planning

The wedding site uses Actix Web, Askama templates, and PostgreSQL 17.

## Running locally

Install stable Rust and start PostgreSQL with `docker compose up -d postgres`.
From the repository root, configure `DATABASE_URL` to point to your local wedding
database and run `cargo run -p api`. The server defaults to port 8080; override
it with `PORT`. Database migrations run automatically at startup. Static assets
are resolved relative to the repository root.

To enable admin access, set **both** `ADMIN_USERNAME` and `ADMIN_PASSWORD` in the
server environment before starting it. Use a unique, strong password; do not
commit credentials or put them in URLs. An unset pair disables all admin access
while keeping public pages available. Partial or empty configuration prevents
startup, and the username cannot contain `:`.

## RSVP

`/rsvp` accepts a name, email address, attending/declining choice, and party size.
Attending parties may contain **1 or 2 people, including the submitter**.
Declines always record zero attendees. The form works without JavaScript.

Email identity is trimmed and case-insensitive. The first response wins, even
for simultaneous submissions; later submissions neither replace it nor rename
the guest. Guests must use `/contact` to request changes. A pre-existing guest
without an RSVP is reused without changing its stored name or email.
This public form does not verify email ownership or enforce invitation access.
Meal preferences, confirmation emails, and self-service editing are not included.

### Administration and API access

Open `/admin/rsvps` and enter the configured HTTP Basic credentials when
prompted. This read-only page lists names, emails, attendance, party sizes,
and submission times in UTC, with total responses and total attending people.
It is not linked from public navigation and is served with `Cache-Control:
no-store`.

**Authentication change:** `GET /guests`, `POST /guests`, and
`POST /guests/{guest_id}/rsvps` now require the same Basic credentials. Existing
API clients must send an `Authorization` header. Public RSVP submissions go
through `POST /rsvp`, not the guest APIs.

The guest-specific JSON RSVP API accepts `attending`, optional `party_size`,
and the existing optional `meal_preference`. An omitted party size defaults to
1 when attending or 0 when declining. Invalid counts return 400, unknown guests
return 404, and an existing response returns 409. The guest creation API trims
names, normalizes emails, and returns 409 for an existing email.

**Production requires HTTPS.** Basic authentication does not encrypt
credentials. Render terminates HTTPS in front of this application; use only
HTTPS URLs for admin/API access and do not expose the application port directly
to the public Internet. `render.yaml` declares manually supplied admin secrets
(`sync: false`); configure these in Render before using administration.
Local HTTP is only appropriate on a trusted development machine.

### Migrating existing data

Migration 003 adds party size and database uniqueness constraints. Historical
attending responses become 1 attendee and declines become 0; a guest's old
`plus_one` flag is not treated as an attendance count.

If multiple responses exist for one guest, or guest emails collide after
trimming/lowercasing, startup stops with a remediation message. Back up the
database and reconcile those records manually before restarting. The migration
does not delete responses or merge guest identities automatically.

## Checks

Run formatting, tests, and the repository lint alias:

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo lint
```

Database-backed tests require `DATABASE_URL` pointing to an **isolated local
PostgreSQL instance** with permission to create databases. SQLx creates and
cleans separate test databases; never point this command at a deployed or
shared production database. For example, start an isolated test container:

```powershell
docker run --rm -d --name wedding-rsvp-tests -p 127.0.0.1:55432:5432 -e POSTGRES_HOST_AUTH_METHOD=trust postgres:17
$env:DATABASE_URL = "postgres://postgres@localhost:55432/postgres"
cargo test --workspace
docker stop wedding-rsvp-tests
```

The trust-authenticated test database is for disposable local data only. Wait
for `docker exec wedding-rsvp-tests pg_isready -U postgres` to succeed before
running tests. Tests cover legacy migrations, atomic/concurrent submissions,
validation and escaping, duplicate handling, authentication, admin totals,
and the JSON API's backward-compatible defaults.