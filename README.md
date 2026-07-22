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
