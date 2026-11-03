# Security policy

## Reporting a vulnerability

Report security problems privately through GitHub's vulnerability reporting:
[github.com/lw-xiong/Silica/security/advisories/new](https://github.com/lw-xiong/Silica/security/advisories/new).
Do not open a public issue, pull request or discussion for one.

Please include the Silica version (or commit), your platform, how to reproduce
it — a vault, a note, a config file or a socket request that triggers it — and
what an attacker gains.

What to expect: an acknowledgement within 7 days, a fix in a tagged release,
and credit in its notes unless you would rather not be named. Please allow 90
days, or until the fix ships, before disclosing publicly; if a fix needs longer
you will hear why and we agree a date.

## What Silica promises

Silica opens a folder and treats it as the only thing it may touch. These are
the properties a report is measured against:

- **Writes stay inside the vault.** Every write goes through one checked path
  (`Vault::write_path`): a `..` or an absolute path is refused, so is a
  symbolic link anywhere along the way, and the file is opened without
  following a link in its place (`O_NOFOLLOW` on Unix). On Unix the write
  walks the path with `openat` from the vault root rather than naming it, so
  there is no window between the check and the write in which a directory can
  be replaced by a link; directory creation walks the same way. The only
  writes outside the folder are a file moved to the system trash when
  `trashOption` asks for it, and nothing else.
- **Network access happens only for a remote image you can see.** The app
  installs an HTTP client (`src/http_client.rs`) and the reading view asks it
  for every remote image a note displays (`EmbedKind::RemoteImage`), through
  the host's `curl`, with a 30-second limit, following redirects, caching the
  bytes under `$XDG_CACHE_HOME/silica/remote_images`. A remote image inside
  code, a link, or a note that is not open is not fetched. Nothing else in the
  app spends the client: a web link still opens in the system browser only
  when clicked, and no telemetry, update check or feed request exists.
  What a fetch therefore reveals to the image's host is your IP address and
  the time you opened that note, plus the URL itself.
  Destinations and size are bounded: only `http` and `https` are fetched, at
  most five redirects are followed and only to those two schemes, each
  redirect hop is revalidated against the same private-network rules before
  the connection is made (so a redirect chain cannot bounce to a private
  address the original URL passed), a literal
  loopback, link-local, carrier-grade-NAT or private address is refused before
  any request (`localhost`, `127.0.0.0/8`, `10/8`, `172.16/12`,
  `192.168/16`, `169.254/16`, `100.64/10`, `fc00::/7`, `fe80::/10`, `::1`),
  and a response past 32 MB is dropped instead of cached. A DNS name is
  resolved before connecting and refused if every address it could reach is
  private, and curl is pinned to the validated address with `--resolve`, so
  DNS rebinding cannot reach the local network through a look-alike name.
  A name that cannot be resolved is not fetched.
- **Code from the vault runs only when you enable a community plugin.** Themes,
  CSS snippets, manifests and settings are read as data and never evaluated.
  An *enabled* tier-1 plugin is different: `silica-plugins` loads its
  `main.js` into a QuickJS runtime and runs it (`src/host.rs`). What that
  runtime can do:
  - **Filesystem** — read and write paths inside the vault only. Every path is
    resolved against the vault root, refused if it contains `..` or a symbolic
    link anywhere along the way, and written through `openat` with
    `O_NOFOLLOW` on every component, so no check-then-open race can redirect
    a write outside the vault. Writes to the note open in the editor are
    refused so unsaved edits cannot be lost, and a delete moves the file to
    `.trash` rather than unlinking it.
  - **Network** — none. The plugin runtime has no network primitive, and the
    app's HTTP client is not exposed to the shim.
  - **Process, threads, dynamic code from outside** — none. No `eval` of
    fetched text, no shell, no `require`.
  - **Budgets** — 64 MB heap, 5 seconds to load, 10 seconds per command;
    a plugin that exceeds one is stopped with an error.
  - A plugin cannot write the app's own binary, cannot write outside the vault,
    and cannot reach the control socket.
  Disabling a plugin stops it being loaded at all.
- **Links open only schemes a link is meant to carry.** `http`, `https` and
  `mailto` are handed to the desktop's opener; anything else — `file:`,
  `ms-msdt:`, `search-ms:`, `javascript:`, `data:`, `obsidian:` — is refused
  before the opener sees it, because a URL is a program name to a desktop.
  No shell is ever in the path: Windows uses `ShellExecuteW`, not
  `cmd /C start`, so `&` and `|` in a URL are characters rather than syntax.
- **The control socket is local and private.** `--control <path>` binds a Unix
  domain socket with mode `0600`; it accepts no path that leaves the vault.
  It is off unless asked for, and it is as powerful as the keyboard — anyone
  who can connect to it can edit the vault.
- **Bytes it did not change are left alone.** Config files keep their unknown
  keys, BOMs and line endings; a note that changed on disk underneath an open
  buffer is not overwritten. A note that *is* overwritten is saved atomically
  (temp file, `fsync`, `rename`) and the bytes it replaces are snapshotted to
  `.trash/.snapshots/` first, with a nanosecond-and-counter filename so two
  saves in one second cannot overwrite an earlier snapshot. Snapshots and
  their directory are written through the same `openat` walk as vault writes,
  so a symlink in the path cannot redirect them outside the vault.

## Scope

In scope: the `silica` binary and its release archives, `silica-mcp`, the
control socket, and anything a malicious vault, note, config file, canvas or
link can make the app do — write outside the folder, run code, leak data,
crash with a buffer of unsaved edits.

Out of scope: problems that need an attacker who already controls your user
account or the vault folder's permissions, and the behaviour of Obsidian
itself.

## Independent verification

The project's own test suite covers the security fixes with unit tests and
graphical end-to-end tests. What it does not cover is a review by someone
outside the project. That independent audit remains outstanding; no external
security review has been commissioned or completed.

Until then, every push is verified by CI on Linux, Windows, and macOS: the
workspace suite, `cargo clippy -D warnings`, a supply-chain audit
(`cargo audit`), and graphical end-to-end tests driven against the real
binary over its control socket under Xvfb. The Windows job is the one that
proves the non-`openat` write path holds without Unix primitives.

## Supported versions

Only the latest tagged release gets fixes. The binaries are built by the
release workflows in this repository; see [README](README.md#releases) for how
to verify a download.
