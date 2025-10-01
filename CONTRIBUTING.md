# Contributing to ObsyncGit

Thanks for your interest in improving ObsyncGit! This guide explains how we organise branches, how releases are produced, and what we expect from pull requests.

## Branching model

- `develop` is the integration branch; every change is merged here first via pull requests.
- `main` mirrors `develop`. A dedicated workflow fast-forwards `main` after every successful push to `develop`, so do **not** open manual PRs from `develop` to `main`.
- Feature branches must start with an approved prefix (`feature/`, `bugfix/`, `hotfix/`, `chore/`, `docs/`, `refactor/`, `ci/`, `build/`, `test/`).
- Hotfix work may branch directly from `main`, but still merge back into `develop` so the automation can promote it forward.
- After a pull request is merged, the source branch is deleted automatically to keep the branch list tidy.

All changes enter the codebase via pull requests. Direct pushes to `develop` are reserved for automation; never force-push to protected branches.

## Workflow for contributors

1. Fork the repository and clone your fork.
2. Branch from `develop`, e.g. `git checkout -b feature/add-awesome-thing develop`.
3. Implement your changes and keep commits tidy using [Conventional Commits](https://www.conventionalcommits.org/). Squash or rebase before pushing so the final commits are conventional.
4. Run the quality gates locally:
   - `cargo fmt`
   - `cargo clippy --all-targets --all-features`
   - `cargo check --all --all-features --locked`
   - `cargo test --all --all-features --locked`
   - `shellcheck scripts/install.sh`
   - `pwsh -NoProfile -Command "Set-ExecutionPolicy -Scope Process Bypass -Force; Import-Module PSScriptAnalyzer; Invoke-ScriptAnalyzer -Path scripts/install.ps1 -Recurse -Severity Error"`
5. Push your branch and open a pull request targeting `develop`. The PR title must follow the Conventional Commit format; CI enforces this rule.
6. Ensure the GitHub Actions CI checks are green and request review.

### Pull request guidelines

- Keep PRs focused; prefer multiple smaller PRs over one large refactor.
- Update documentation and tests alongside code changes.
- Add release notes context when relevant (the `CHANGELOG.md` is maintained automatically but commit messages fuel it).
- Resolve review feedback promptly; rebase onto the latest `develop` if conflicts appear.

## Release flow

Releases are automated with [release-please](https://github.com/googleapis/release-please).

1. Every push to `develop` runs `Promote Develop to Main`, which fast-forwards `main` to the same commit (failures require a maintainer to resolve divergences).
2. Each push to `main` triggers `Release Please`. As soon as at least one user-facing Conventional Commit (`feat`, `fix`, `docs`, `chore`, `ci`, `refactor`, `build`, `style`, `test`, `revert`, `hotfix`) lands, release-please opens a release PR (`chore(main): release x.y.z`).
3. Review and merge the release PR. release-please creates the git tag and GitHub Release, which starts the `Release` workflow to publish binaries for Linux, macOS, and Windows.
4. The `Sync Release Back to Develop` workflow fast-forwards `develop` after a release commit so both branches stay aligned.

> **Maintainers:** The `Release Please` workflow requires a classic personal access token (PAT) with `repo` scope stored as the `RELEASE_PLEASE_TOKEN` secret. This bypasses GitHub's restriction on workflows creating pull requests. Generate the PAT from your account and add it under *Settings → Secrets → Actions*.
>

> Release detection relies on Conventional Commits; malformed commit messages block releases. Ensure the final commits that land on `develop` (typically via squash merge) follow the format before approval.


## Development environment

- Rust stable toolchain (configured automatically in CI via `dtolnay/rust-toolchain@v1`).
- Dependencies are managed by Cargo; avoid committing binaries or build artefacts.
- Use `scripts/install.sh` / `install.ps1` for manual testing of the installer flows.

## Code style

- Rustfmt enforces formatting; run `cargo fmt --all` before committing.
- Lints must pass with `cargo clippy --all-targets --all-features -- -D warnings`.
- Tests must pass on all tiers; add coverage for new behaviour.
- Shell scripts are linted with [ShellCheck](https://www.shellcheck.net/); PowerShell scripts run through [PSScriptAnalyzer](https://github.com/PowerShell/PSScriptAnalyzer).

## Repository configuration

- Enable **Automatically delete head branches** in *Settings → General → Pull Requests* so merged branches disappear automatically (the workflow in this repo also deletes them as a fallback).
- Add branch protection rules via *Settings → Branches*:
- `main`: require status checks (`CI`, `Branch Builds`, `Release Please`), block force-pushes/deletions. Direct PRs should remain disabled; automation pushes fast-forwards only.
- `develop`: require pull requests, at least one approval, dismiss stale reviews, require CODEOWNERS review, block force-pushes/deletions, require status checks (`CI`, `Branch Builds`), and enforce conversation resolution.
- Keep workflow permissions at the default of “Read and write” and avoid granting bypass rights except to trusted maintainers.

## Reporting issues

Use GitHub Issues to report bugs or request features. Describe:
- Steps to reproduce
- Expected vs actual behaviour
- Environment details (OS, Rust version)
- Logs or stack traces if available

Thanks again for contributing!
