# Charlotte naming compatibility

The product, source code, Electron preload bridge, service unit, build assets
and repository documentation now use **Charlotte**.

Two historic string values are deliberately not rewritten because they are
externally persisted resources rather than file or symbol names:

- `paperkite-browser`: previously used Electron profile directory. The
  main process reads it only to preserve existing bookmarks and sessions
  when users upgrade. Fresh installations use Charlotte's default directory.
- `paperkite-chat-service.samuelabolo.dev`: active external chat endpoint.
  Changing this domain in code would break connectivity without a matching
  deployed hostname and TLS certificate.

For Linux deployments, a legacy configuration-location check is retained
in the provisioning script, and a migration plan lives in
[server/deploy/MIGRATION.md](server/deploy/MIGRATION.md).

The name of a compiled file or SVG asset does not change the artwork
inside it. Visual identity redesign, DNS cutover and running-service
migration are separate operational work.
