# Baicebo Wedding Planning

The wedding site uses Actix Web, Askama templates, and Notion for RSVP storage.

## Running locally

Set `NOTION_TOKEN` and `NOTION_RSVP_DATA_SOURCE_ID` in the server environment. To keep progress correct after manual Notion changes (including deletions), create a Notion connection webhook for the RSVPs data source. Use `https://baicebo.com/webhooks/notion/rsvp-progress?token=<NOTION_PROGRESS_WEBHOOK_SECRET>` as its URL and subscribe to `page.created`, `page.properties_updated`, `page.deleted`, and `page.undeleted`. The endpoint also accepts a database automation using an `X-Notion-Progress-Secret` header.
Share the RSVP database with the Notion integration, including read and insert
capabilities. Keep the token secret and never commit it. The data source ID is
`41b96eec-0111-81c0-9318-000bf13b5594` for the current wedding workspace.

Run `cargo run -p api` or `docker compose up --build`. The server listens on
port 8080 by default. Docker Compose reads the two Notion variables from the
shell; the data source ID has the default above. Without these variables, public
pages are available but RSVP submissions show an error.

## RSVP

Each guest, including a plus-one, submits a separate response. Names, email,
attendance, phone, optional accommodation, dietary needs and main-course choice are
saved directly to Notion. The guest sees a confirmation only after Notion
accepts the response. If a response cannot be confirmed, the form asks the guest
to contact the couple before submitting again. Guests sharing an email address
may respond separately. There is no website admin page; view responses in the
Notion RSVP database.

Each form has a submission ID. The server checks Notion for that ID before
creating a page and serializes submissions within each server process. Notion
does not provide an atomic uniqueness constraint, so this is not an exactly-once
guarantee across server instances or a restart after an uncertain request.

## Existing production data

The previous PostgreSQL database remains for migration only; the website no
longer connects to it. Existing production RSVPs have not yet been migrated to
Notion. Do not remove the Render database or deploy a cutover
until the database is backed up and all existing responses have been copied and
reconciled against Notion, including historical party sizes. The database
resource stays in `render.yaml` to avoid deleting production data.

## Checks

Run `cargo fmt --all -- --check` and `cargo test -p api`.
