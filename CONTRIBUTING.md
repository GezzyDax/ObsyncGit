# Contributing to ObsyncGit

Thanks for your interest in improving ObsyncGit! This guide explains how we organise branches, how releases are produced, and what we expect from pull requests.

## Branching model

We follow a Gitflow-inspired model:

- `main` always reflects the latest production-ready release.
- `develop` is the integration branch; all new work lands here first.
- Feature work happens in short-lived branches prefixed with `feature/`, `bugfix/`, or `hotfix/` as appropriate.
- Urgent production fixes may branch directly from `main` using the `hotfix/` prefix and must be merged back into both `main` and `develop`.

All changes enter the codebase via pull requests. Direct pushes to `main` or `develop` are discouraged.

## Workflow for contributors

1. Fork the repository and clone your fork.
2. Branch from `develop`, e.g. `git checkout -b feature/add-awesome-thing develop`.
3. Implement your changes and keep commits tidy using [Conventional Commits](https://www.conventionalcommits.org/).
4. Run the quality gates locally:
   - `cargo fmt`
   - `cargo clippy --all-targets --all-features`
   - `cargo check --all --all-features --locked`
   - `cargo test --all --all-features --locked`
   - `shellcheck scripts/install.sh`
   - `pwsh -NoProfile -Command "Set-ExecutionPolicy -Scope Process Bypass -Force; Import-Module PSScriptAnalyzer; Invoke-ScriptAnalyzer -Path scripts/install.ps1 -Recurse -Severity Error"`
5. Push your branch and open a pull request targeting `develop`.
6. Ensure the GitHub Actions CI checks are green and request review.

### Pull request guidelines

- Keep PRs focused; prefer multiple smaller PRs over one large refactor.
- Update documentation and tests alongside code changes.
- Add release notes context when relevant (the `CHANGELOG.md` is maintained automatically but commit messages fuel it).
- Resolve review feedback promptly; rebase onto the latest `develop` if conflicts appear.

## Release flow

Releases are automated with [release-please](https://github.com/googleapis/release-please).

1. When `develop` is stable, maintainers create a PR from `develop` into `main` (merging with "merge commit" to preserve individual commits).
2. A successful merge into `main` triggers the "Release Please" workflow, which opens an automated release PR (`chore: release x.y.z`). This PR bumps crate versions, updates `CHANGELOG.md`, and prepares tags.
3. After review, maintainers merge the release PR. release-please creates the git tag and GitHub Release, which in turn triggers `Release` workflow to publish binaries for Linux, macOS, and Windows.
4. Finally, merge the release PR back into `develop` (usually by fast-forwarding `develop` to `main`) to keep branches in sync.

> **Maintainers:** The `Release Please` workflow requires a classic personal access token (PAT) with `repo` scope stored as the `RELEASE_PLEASE_TOKEN` secret. This bypasses GitHub's restriction on workflows creating pull requests. Generate the PAT from your account and add it under *Settings → Secrets → Actions*.

## Development environment

- Rust stable toolchain (configured automatically in CI via `dtolnay/rust-toolchain@v1`).
- Dependencies are managed by Cargo; avoid committing binaries or build artefacts.
- Use `scripts/install.sh` / `install.ps1` for manual testing of the installer flows.

## Code style

- Rustfmt enforces formatting; run `cargo fmt --all` before committing.
- Lints must pass with `cargo clippy --all-targets --all-features -- -D warnings`.
- Tests must pass on all tiers; add coverage for new behaviour.
- Shell scripts are linted with [ShellCheck](https://www.shellcheck.net/); PowerShell scripts run through [PSScriptAnalyzer](https://github.com/PowerShell/PSScriptAnalyzer).

## Reporting issues

Use GitHub Issues to report bugs or request features. Describe:
- Steps to reproduce
- Expected vs actual behaviour
- Environment details (OS, Rust version)
- Logs or stack traces if available

Thanks again for contributing!
