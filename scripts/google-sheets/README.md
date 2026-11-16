# RSVP Google Sheets sync

PostgreSQL remains the source of truth. The API writes every RSVP to PostgreSQL and an outbox row in the same transaction; the worker retries the sheet write until the webhook acknowledges it. Migration 004 also queues a snapshot of every existing RSVP, preserving legacy group responses and party sizes.

## Configure the Apps Script endpoint

1. Open the Wedding RSVP spreadsheet.
2. Choose **Extensions → Apps Script** and replace the editor contents with `Code.gs` from this directory.
3. In **Project Settings → Script Properties**, add:
   - `SPREADSHEET_ID`: `15yjLGp5UShmqsJkFnVaBR0Gj0cSC3khEsIO8A1MxUa0`
   - `SHEET_NAME`: `RSVPs`
   - `SHARED_SECRET`: a newly generated long random secret
4. Deploy the script as a **Web app**, running as your account and accessible to **Anyone**. The shared secret authorizes each request.
5. In the wedding API's Render environment, set:
   - `GOOGLE_SHEETS_WEBHOOK_URL`: the deployed web app's `/exec` URL
   - `GOOGLE_SHEETS_WEBHOOK_SECRET`: the same secret
6. Deploy the API. Pending existing RSVP snapshots and all future submissions will sync automatically. The spreadsheet script updates rows by stable RSVP ID, so retries do not create duplicates.

Do not put the secret in Git or the Apps Script source. The script receives RSVP data over HTTPS and protects it with the shared secret. If the endpoint is temporarily unavailable, submissions remain committed in PostgreSQL and are retried with backoff.
