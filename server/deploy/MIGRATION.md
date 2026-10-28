# Upgrading an existing server to Charlotte paths

New deployments use the **charlotte** system user, `/opt/charlotte`,
`/etc/charlotte/ops.env` and `charlotte-ops.service`.

**Important:** Renaming source paths cannot rename an existing Linux user,
systemd service, on-disk username registry, TLS certificates or DNS record.
An older installation may still be running with the former names. Do not
run `setup-droplet.sh` over such an installation without migrating data.
The provision script deliberately stops if it detects the old configuration
and no new configuration exists.

## Planned migration (existing droplet only)

Do this in a maintenance window. Make an off-host backup and confirm it can
be restored. Check current process state, ownership, ports and current
systemd unit before executing commands.

```sh
# Historical paths are used here ONLY to locate the existing installation.
sudo systemctl stop paperkite-ops.service
sudo systemctl disable paperkite-ops.service

# Backup both the identity signing secrets and permanent username claims.
sudo tar -C / -czf /root/charlotte-legacy-backup.tar.gz \
  etc/paperkite/ops.env opt/paperkite/data

# Create new paths without discarding persistent data.
sudo install -d -m 0700 /etc/charlotte
sudo install -d -m 0755 /opt/charlotte
sudo cp -a /opt/paperkite/data /opt/charlotte/data
sudo cp -a /etc/paperkite/ops.env /etc/charlotte/ops.env
sudo sed -i 's#/opt/paperkite#/opt/charlotte#g' /etc/charlotte/ops.env
sudo chmod 600 /etc/charlotte/ops.env
```

Run the **reviewed** `server/deploy/setup-droplet.sh` from the Charlotte
repository after confirming the above copies and your GitHub release
credentials. The script creates the Charlotte user, fixes ownership, sets
up the new unit and restarts under the new paths. Validate `/healthz`,
the persistent username registry and chat reconnections before retiring
the legacy paths. If the GitHub repo is private, anonymous release asset
downloads may fail: arrange authenticated downloads or a separately built
binary first.

Do not rotate `CHAT_JWT_SECRET` or `OPS_JWT_SECRET` as part of this path
rename unless you intentionally want to invalidate existing sessions
and deployment tokens. Keep the legacy backup until recovery is tested.
Do not run old and new units at the same time (both bind the same ports).

## Legacy public hostname

The configured production chat hostname
`paperkite-chat-service.samuelabolo.dev` is **an external DNS/TLS address**,
not an application file. It remains in the browser and site as a working
compatibility endpoint until a new Charlotte hostname has real DNS,
certificates and hosting.

To switch after that work is complete, set
`CHARLOTTE_CHAT_SERVER_URL` for the desktop browser and
`NEXT_PUBLIC_CHARLOTTE_CHAT_SERVER_URL` when building the static site,
then verify health and chat connectivity. Existing saved per-server
connections may continue using their previous base URL; migrate those
explicitly rather than invalidating profiles.
