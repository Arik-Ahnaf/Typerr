# Startup updates

Typerr 2.0.0 checks `Arik-Ahnaf/Typerr`'s latest published stable GitHub release
at most once each launch. It compares semantic versions, excluding prereleases and
older/equal versions. The legacy v1.0.0 release packaged a 0.1.0 binary; 2.0.0
corrects that mismatch. Install 2.0.0 manually first. No release is published
by building this feature.

The 10-second background check never blocks typing. Offline, malformed, rate-
limited, and missing-asset responses are logged without opening a dialog or
printing to the shell. HTTP 403/429 responses defer subsequent launch checks
until GitHub's `x-ratelimit-reset` or `Retry-After` deadline (whichever is later),
with a one-hour fallback when neither provides a usable deadline. A minimum
60-second cooldown prevents immediate retries. This worker-owned deadline is
stored in `update-check-retry-at` beside `scores.db`, independently of settings;
checks resume on the next launch after it expires. A denied check cannot
determine whether an update exists. No GitHub token is required or stored.
An available update waits until the test and fade finish, Dashboard opens, or
Tab starts a fresh idle session. Later dismisses it for this launch. No preference
or settings write is made. Tab/Shift+Tab switches dialog buttons, Enter/Space
activates the selected button, and Escape chooses Later/Close/Cancel when allowed.
The initial selection is Later to avoid accidental installation.

Upgrade and restart downloads and verifies the selected platform artifact. It
shows progress, with Cancel until installation begins. GUI acknowledgment makes
the transition to installation explicit, after which closing the window is
suppressed until completion or failure. Installation failures offer Retry/Close.

## Integrity and bounds

Only GitHub HTTPS API/download URLs and known GitHub release CDN hosts are
accepted; TLS certificate verification remains enabled. Redirects are individually
checked, limited to five, and share the request time budget. Checks are capped
at 1 MiB of JSON, SHA256SUMS at 64 KiB, archives at 128 MiB, and extraction at
512 MiB / 10,000 entries. Downloads have a 10-minute deadline and 30-second read
limit. The SHA256SUMS entry is mandatory; GitHub SHA-256 digests are additionally
checked when supplied. These checks trust the GitHub release publisher and HTTPS;
they are not independent publisher signatures.

Archive roots must match the release version and platform. Reject links, special
files, duplicate/case-colliding names, paths outside the expected package, and
unexpected payloads. Verify native executable architecture and its side-effect-
free `--version` output before replacement. New packages must retain this CLI and
helper/confirmation compatibility. Network and archive operations run off the UI
thread; the SQLite worker remains dedicated to score storage.

## Portable copies

Portable installation needs a writable application folder. Staging and journal
are in `.typerr-update` beside the executable. Per-installation OS file locks
under the user data `updates/` directory prevent simultaneous updates and reject
updates while another Typerr instance is open for the same user/installation.

The copied helper waits for normal app shutdown, including queued score writes.
It backs up and replaces only shipped top-level files/directories. Existing
regular files are copied to durable backups before atomic replacement, keeping
the executable available at its installed path throughout the commit. Existing
`settings.json`, score databases, and unrelated files remain untouched. The
notices directory is application-owned. The original working directory is used
for restart. The new version confirms startup on its first native redraw; until
then the helper keeps backups. Failed replacement/startup restores the previous
files. Interrupted work is recovered on next launch; Windows uses a separate
recovery process to avoid replacing the running executable. Completed Windows
helper/staging files are cleaned on a later launch.

Do not delete `.typerr-update` during an update. If interruption leaves the main
executable unavailable between renames, run the retained `helper`/`helper.exe`
from a terminal with `--typerr-recover <absolute-install-folder>
<absolute-install-folder>/.typerr-update`. On Windows quote paths containing
spaces. Keep the directory intact until recovery completes. If recovery reports
an invalid journal, retain backups and reinstall the portable archive manually;
do not overwrite your existing settings. No database rollback/migration is part
of an application update.

## Pacman installations

Ownership of the resolved executable is queried with pacman. A `typerr`-owned
executable uses the matching `.pkg.tar.zst`; other package ownership is refused.
Unowned `/usr` and `/opt` installations require manual updates. The updater
checks package name, version, and x86_64 architecture with `pacman -Qip`, then
uses `/usr/bin/pkexec --disable-internal-agent /usr/bin/pacman -U --noconfirm`
with the verified package as a separate argument. Only pacman runs as root;
the application/helper retain the original user. Polkit must have a graphical
authentication agent. No shell command is constructed from release data.

Pacman handles package transactions, dependencies, signature policy, and settings
backup; the updater never changes its policy or directly restores package-owned
files. Authentication cancellation, database locks, dependency conflicts, and
signature failures are reported. Failed package downloads/installations remain
under the user data `updates/package-*` directory for inspection/manual install;
remove abandoned directories when no updater is running. If a package installs
but restart fails, restart Typerr manually; package rollback is a pacman operation.

## Diagnostics and verification

Background check errors go only to `updates-errors.log` beside `scores.db`;
installation/recovery errors also go to stderr. Updater files
contain release metadata, package payloads, and installation paths, not typing
history or target text. No GitHub token is required or stored.

Automated tests cover selection, modal isolation/deferral, package metadata,
file locks, and replacement/recovery using temporary fixtures. Run native UI and
end-to-end restart checks separately. Windows cross-compilation does not verify
Windows locks, executable replacement, UAC, or graphical startup. Real privileged
pacman upgrades belong in a disposable Arch environment, never the developer's
host installation. Dated evidence and remaining limitations belong in PLAN.md.
