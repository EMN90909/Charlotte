# Charlotte server

The `server/` directory contains the Go chat backend formerly in `chat-service/`, with internal imports updated to `github.com/EMN90909/Charlotte/server`. Existing public server addresses and the chat-token protocol are not automatically changed.

## Supabase account authentication

The browser offers **email/password account sign-in and sign-up** through Supabase Auth. The Electron main process handles credentials and stores refresh tokens using OS-backed encryption when available. No secret or service-role key belongs in the client.

Configure the **same** Supabase project in the browser and this server:

```sh
# Server environment
SUPABASE_URL=https://YOUR_PROJECT.supabase.co
SUPABASE_PUBLISHABLE_KEY=sb_publishable_YOUR_KEY
CHARLOTTE_WEB_ORIGIN=https://charlotte-9wyy.onrender.com
CHAT_JWT_SECRET=replace-with-long-random-existing-chat-secret
```

The endpoint `GET /auth/me` accepts `Authorization: Bearer <Supabase access token>`, calls Supabase `/auth/v1/user` to verify it server-side and returns only the user id and email. It returns 401 for expired/invalid tokens and 503 when Supabase is not configured. There is **no** service-role key, custom JWT signing, or user-profile table.

The existing chat identity/username-token flow remains independent for backwards compatibility; logging into Charlotte does **not yet** make chat rooms require Supabase authentication. Enable that only as a separate, protocol-aware authorization migration.

For the static website, `CHARLOTTE_WEB_ORIGIN` narrowly enables browser-side CORS for `/healthz` and `/custom-metrics`. No browser-origin CORS is granted to protected auth endpoints.

## Verify

```sh
cd server
go test ./...
go build ./cmd/server
```

Use a test Supabase project and a confirmed email account to verify full login/signup/refresh/signout and server identity validation. Preserve the original [MIT licence](LICENSE).
