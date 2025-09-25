# ObsyncGit

Lightweight watcher daemon that keeps a working tree in sync with a remote Git repository. It runs in the background, turns local edits into small commits, and periodically pulls remote changes made on your other machines.

## Features
- Watches a directory (recursively) and debounces edits before staging them.
- Generates readable commit messages such as `auto: Issues.md` or `auto: updated 4 files`.
- Rebases onto the remote branch before every push to minimise merge conflicts.
- Periodically pulls remote changes even when nothing happens locally.
- Configurable ignore globs so editor caches and similar junk stay out of Git.
- Emits compact structured logs via `tracing`.

## Build & Run
```bash
cargo build --release
./target/release/obsyncgit --config /path/to/config.yaml
```

To run once (for testing) simply stop with `Ctrl+C`. The daemon shuts down cleanly.

### Install as a systemd user service (Linux)
1. Copy the release binary somewhere on your `$PATH`, e.g. `~/.local/bin/obsyncgit`.
2. Copy the supplied unit file and adjust the paths:
   - `cp examples/obsyncgit.service ~/.config/systemd/user/`
   - edit the `ExecStart` and `WorkingDirectory` lines.
3. Reload and enable:
   ```bash
   systemctl --user daemon-reload
   systemctl --user enable --now obsyncgit.service
   ```
4. Inspect logs with `journalctl --user -u obsyncgit -f`.

macOS users can adapt the binary for `launchd` (see `examples/obsyncgit.plist`) and Windows users can register it through Task Scheduler or `nssm`.

## Configuration
Create a YAML file (see `config.example.yaml` in the repo). All paths must be absolute.

```yaml
repo_url: "git@github.com:you/vault.git"
branch: "main"
remote: "origin"
workdir: "/home/you/Obsidian"
debounce_seconds: 5
poll_interval_seconds: 180
commit:
  prefix: "auto:"
  max_files_in_summary: 5
  include_timestamp: true
ignore:
  globs:
    - ".obsidian/cache/**"
    - "**/*.tmp"
self_update:
  enabled: false
  command: null
  interval_hours: 24
git:
  author_name: "Vault Sync"
  author_email: "sync@example.com"
```

Field notes:
- `repo_url`: SSH or HTTPS remotes work. The daemon runs `git remote set-url` if needed.
- `workdir`: Must either be an empty directory or an existing clone of `repo_url`.
- `debounce_seconds`: Minimum idle time before a commit is attempted.
- `poll_interval_seconds`: How often to `git pull --rebase` when no local edits happen.
- `commit.max_files_in_summary`: controls how many filenames appear in commit messages. Above that limit the message switches to `updated N files`.
- `ignore.globs`: Standard glob patterns matched against paths relative to `workdir`.
- `self_update`: Optional hook; when `enabled` and `command` is provided the daemon will execute the command on its own schedule (hook stub provided in code).
- `git`: Optional overrides for author/committer identity.

## Behaviour details
- New files are automatically staged thanks to `git add -A`.
- Commits are only produced when `git status --short` reports changes. If nothing is pending the daemon just performs periodic pulls.
- On rebase conflicts the daemon aborts the rebase and backs off exponentially; manual intervention is then required.
- Git commands run with `GIT_TERMINAL_PROMPT=0`, so configure SSH keys/credentials beforehand.

## Troubleshooting
- Run with `OBSYNCGIT_LOG=debug` to see every git invocation.
- Ensure the repository has sane permissions; the daemon does not sudo or elevate.
- Large binary files should be excluded with `.gitignore` or added to `ignore.globs`.

## Roadmap hooks
`SelfUpdateConfig` and update hooks are wired into the configuration so you can extend `SyncDaemon::event_loop` with custom behaviour (e.g. invoke a release script or fetch the latest binary from a GitHub release).
