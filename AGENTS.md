# Repository Guidelines

## Project Structure & Module Organization
Runtime code sits in `src/`, where `main.rs` wires the CLI and `lib.rs` exposes shared services. Support modules (`git.rs`, `daemon.rs`, `updater.rs`, `ignore.rs`) isolate sync stages. The desktop helper (`obsyncgit-gui`) lives in `src/bin/` with its Slint markup in `ui/`. Installer assets are stored under `scripts/`, sample service files under `examples/`, and brand collateral in `assets/`. Configuration templates (`config.example.yaml`) exist for reference only—never commit edited copies.

## Build, Test, and Development Commands
Use `cargo build --all-features` during feature work and `cargo build --release` before packaging. Validate code quickly with `cargo check --all --all-features --locked`. Format and lint through `cargo fmt --all` and `cargo clippy --all-targets --all-features -- -D warnings`. Run `cargo test --all --all-features --locked` before every push. Keep installer scripts healthy via `shellcheck scripts/install.sh` and `pwsh -NoProfile -Command "Import-Module PSScriptAnalyzer; Invoke-ScriptAnalyzer -Path scripts/install.ps1 -Recurse"`.

## Coding Style & Naming Conventions
`rustfmt` defines layout (4-space indent, trailing commas). Use snake_case for functions, variables, and file names; prefer UpperCamelCase for public types and enums. Modules should describe behaviour, e.g. `updater.rs` rather than `utils.rs`. Log fields follow lowercase_with_underscores for consistent `tracing` output. YAML keys stay kebab-case to mirror shipped configs, and Slint component IDs remain dash-separated.

## Testing Guidelines
Co-locate unit tests inside their modules under `#[cfg(test)]` and mirror the namespace being exercised. Add cross-module scenarios as needed under a new `tests/` directory. Focus coverage on config parsing, repository state transitions, and the updater watchdog. Use `cargo test --all --all-features --locked` regularly; add `-- --nocapture` when inspecting watcher output. For manual verification, run binaries with `RUST_LOG=debug` to surface structured traces.

## Commit & Pull Request Guidelines
Follow Conventional Commits (`type(scope): summary`), matching existing entries such as `docs:` and `chore:`. Keep commits focused and avoid unrelated refactors. Create branches from `develop` with one of the approved prefixes (`feature/`, `bugfix/`, `hotfix/`, `chore/`, `docs/`, `refactor/`) so automation accepts the PR. Open pull requests against `develop`, reference linked issues, and summarise user-facing changes plus risk areas. Include CLI logs or GUI screenshots when behaviour shifts. Confirm all checks listed above before requesting review; CI mirrors this matrix.

## Security & Configuration Tips
Scrub credentials from `config.yaml` before sharing logs or patches and prefer environment overrides during testing. When editing installer scripts, verify each distro command path and note changes in `CHANGELOG.md`. Avoid logging repository URLs with embedded secrets, and document any dependency bumps performed with `cargo update` in a dedicated `chore(deps)` change.
