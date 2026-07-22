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

Real Google SSO replaces `dev-login` in a later phase; see `backlogs/`.
