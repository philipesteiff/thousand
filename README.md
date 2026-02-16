# Thousand CLI

`thousand` orchestrates two explicit non-interactive phases for autonomous repo improvements:

1. `find`: identify one finding and open a GitHub issue.
2. `solve`: implement one labeled finding issue and open a PR.

## Prerequisites
- `codex` CLI installed on `PATH`
- `git` configured with `origin` in the target repo
- `THOUSAND_GH_TOKEN` (fallback: `GITHUB_TOKEN`)

## Install
Homebrew (macOS + Linuxbrew x86_64):
```bash
brew tap philipesteiff/tap
brew install thousand
thousand version
```

## Command Contract
- `thousand find --config .thousand/workflow.yaml`
  - Runs scan agent.
  - Creates one issue in the configured repo.
  - Always applies label `thousand` (plus configured labels).
  - Prints the created issue URL.

- `thousand solve --config .thousand/workflow.yaml --issue <number|url>`
  - Accepts issue number (e.g. `7`) or URL.
  - Requires issue to be in the same configured repo.
  - Requires issue label `thousand`.
  - Runs change agent, applies patch, pushes branch, opens PR.
  - Posts an issue comment with the PR URL.
  - Prints the created PR URL.

- `thousand validate --config .thousand/workflow.yaml`
  - Validates workflow schema and required fields.

- `thousand version`
  - Prints CLI version.

## Minimal E2E
```bash
thousand validate --config .thousand/workflow.yaml
thousand find --config .thousand/workflow.yaml
# capture issue URL or number from output
thousand solve --config .thousand/workflow.yaml --issue 7
```

## Release (Maintainers)
Tagged releases (`v*.*.*`) trigger `.github/workflows/release-homebrew.yml`:
1. Runs `fmt`, `clippy`, and `test`.
2. Builds and publishes:
   - `thousand-darwin-arm64.tar.gz`
   - `thousand-darwin-x86_64.tar.gz`
   - `thousand-linux-x86_64.tar.gz`
3. Updates `Formula/thousand.rb` in `philipesteiff/homebrew-tap` using `HOMEBREW_TAP_TOKEN`.

Local packaging smoke test:
```bash
just package
tar -tzf dist/thousand-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m | sed 's/aarch64/arm64/;s/amd64/x86_64/').tar.gz
```
