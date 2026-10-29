# Deploying to a droplet

[`setup-droplet.sh`](setup-droplet.sh) is a DigitalOcean cloud-init
script: paste it into a new droplet's "User Data" field exactly as-is
(see "Secrets" below - no editing needed) and it provisions a fully
running install -
creates an unprivileged `charlotte` user, downloads the latest
`chat-service-ops-linux-<arch>` **and** `chat-service-server-linux-<arch>`
binaries from this repo's GitHub Releases, installs the ops daemon as the
systemd service in [`charlotte-ops.service`](charlotte-ops.service), and
starts it.

On that first start, the ops daemon adopts the already-downloaded server
binary as its initial running version (`OPS_INITIAL_BINARY` in
`ops.env` - see `Manager.Bootstrap` in `internal/ops`), so within ~10-15s
of boot both `charlotte-ops` (`:9090`) and `chat-service-server`
(`:8080`/`:50051`) are actually running - not just ops waiting on a
webhook.

The server binary is **not** started as its own systemd service - the ops
daemon launches it as its own child process, same as it would for a
normal deploy, so a *later* real `/deploy` webhook can find and terminate
it cleanly instead of racing an independently-started process for the
same ports. From that point on, updates work exactly the normal way: push
a `chat-service-v*` tag (with `OPS_WEBHOOK_URL` configured on GitHub, see
`../packaging/README.md`) and CI deploys the new version for you, or
`POST` to `:9090/deploy` by hand.

Already have a droplet running? SSH in as root and run the script
directly instead of recreating the droplet - it's idempotent (safe to
re-run: `useradd` is skipped if the user exists, files are just
overwritten).

## Updating the ops daemon itself

The normal `/deploy` webhook flow only ever updates `chat-service-server`
- `ops` has no self-update mechanism, since a process replacing its own
running binary isn't something to build casually. To update `ops`, run
[`update-ops.sh`](update-ops.sh) on the server (as root):

```bash
./update-ops.sh                       # latest chat-service-v* release
./update-ops.sh chat-service-v0.1.1   # a specific tag
```

or, from a checkout, `make update-ops` (add `TAG=chat-service-v0.1.1` for
a specific tag) from inside `server/`. It stops `charlotte-ops`,
downloads the new binary, and starts it back up - a few seconds of `ops`
downtime, during which `chat-service-server` (a separate process) keeps
running and serving clients unaffected.

## Secrets

The script needs no editing - paste it into User Data exactly as-is. For
`CHAT_JWT_SECRET` and `OPS_JWT_SECRET` it, in order:

1. Uses the value from the environment, if set (only relevant when
   running it by hand: `CHAT_JWT_SECRET=... OPS_JWT_SECRET=... ./setup-droplet.sh`
   over SSH - User Data has no mechanism to inject separate env vars
   alongside pasted script content).
2. Otherwise reuses whatever's already in `/etc/charlotte/ops.env` from a
   prior run - so re-running this script (e.g. to update to a newer
   release manually) never rotates secrets out from under a
   `OPS_JWT_SECRET` you've already put in GitHub.
3. Otherwise generates a fresh one (`openssl rand -hex 32`).

Whatever it ends up using, it prints both values at the end of the run.
If you pasted this as User Data (so you never saw that output live),
retrieve it from the droplet's cloud-init log:
`cat /var/log/cloud-init-output.log`, or the "Droplet Console" in the
DigitalOcean dashboard. Either way, **copy `OPS_JWT_SECRET` into this
repo's GitHub secret of the same name** - it has to match exactly for
the deploy webhook to work, and there's no way to retrieve it again
after this run short of reading `/etc/charlotte/ops.env` on the box
itself (root-readable only, `chmod 600`).

## Domain and TLS (Rust)

The Rust chat server listens over plain HTTP on port 8080 and gRPC on
port 50051. Automatic TLS-ALPN certificates previously supported by the
Go server are not yet implemented in the Rust binary. To use HTTPS,
terminate TLS in a reverse proxy or managed load balancer and forward
HTTP and gRPC requests to their respective local ports.

The ops daemon listens on port 9090; do not expose it publicly. Protect
its signed deploy endpoint with network access controls as well as
OPS_JWT_SECRET.

## Security note: the ops port

`POST /deploy` downloads and *executes* whatever binary URL is in a
validly-signed token - by design, that's what makes auto-deploy work.
That also means anyone who obtains `OPS_JWT_SECRET` can run arbitrary
code on this box. Treat it with the same care as a root SSH key, not like
a casual API token:

- Use a long, randomly-generated secret (`openssl rand -hex 32`, or let
  `setup-droplet.sh` generate one - see "Secrets" above), never something
  memorable.
- It's a GitHub Actions runner calling this endpoint, and GitHub doesn't
  publish a small, stable IP range worth firewalling to - and this setup
  doesn't add rate limiting at the edge either (see "Domain and TLS"
  above). So the JWT signature is the *only* access control here - there
  is no secondary layer softening a leaked or brute-forced secret. Treat
  that as the actual security boundary, not a formality.
- `:9090` has no TLS of its own (see "Domain and TLS" above -
  `PUBLIC_HOSTNAME`/autocert only ever covers the app's `:443`, not the
  ops port), so the deploy webhook's `OPS_JWT_SECRET` bearer token
  currently travels in **plaintext** if `OPS_WEBHOOK_URL` points
  straight at the droplet over `http://`. That's a real gap versus the
  old Caddy-fronted setup, not a stylistic difference - if that token
  leaks off the wire, whoever has it can deploy arbitrary code the same
  as if they'd read it from a log. Until `:9090` gets TLS of its own
  (e.g. a minimal reverse proxy in front of just that port, or a second
  `autocert.Manager` for an ops subdomain), treat any network path
  `OPS_WEBHOOK_URL` traverses as untrusted, and rotate the secret
  proactively rather than only after a known leak.
- Lock down **DigitalOcean Cloud Firewall** to allow public inbound only
  on `:8080`/`:50051`/`:443` (the app) and `:9090` (needed for the
  GitHub Actions webhook - GitHub doesn't publish a small, stable IP
  range worth restricting this to, so it has to stay open to the
  internet generally). Nothing else needs to be reachable.
- Rotate `OPS_JWT_SECRET` (both on the droplet and in the GitHub secret)
  if it's ever exposed in a log, a fork's PR, or anywhere else it
  shouldn't be - and see the plaintext-webhook note above for why that
  risk is higher than it looks.

`:8080`/`:50051`/`:443` (the chat app itself) are meant to be public -
that's the actual product. `:9090` is reachable too (it has to be, for
the webhook) but isn't meant to be treated as a public product surface.
