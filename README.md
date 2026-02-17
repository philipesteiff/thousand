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
  - Runs find agent.
  - Creates one issue in the configured repo.
  - Always applies labels: `thousand`, `severity:<value>`, `type:<value>` (plus configured labels).
  - Prints the created issue URL.

- `thousand solve --config .thousand/workflow.yaml --issue <number|url>`
  - Accepts issue number (e.g. `7`) or URL.
  - Requires issue to be in the same configured repo.
  - Requires issue label `thousand`.
  - Runs solve agent, applies patch, pushes branch, opens PR.
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

## Workflow Config
```yaml
version: 1
name: "security-audit"

query:
  prompt: "Audit the repo for security issues. Create one actionable finding."

agents:
  find:
    model: "gpt-5-codex"
    reasoning_effort: "low" # optional, default: low
  solve:
    model: "gpt-5-codex"
    reasoning_effort: "medium" # optional, default: low

outputs:
  github:
    repo: "owner/repo" # optional; fallback from origin remote
    issue:
      labels: ["thousand"]
    pr:
      draft: true
```

Migration note: `agents.scan` and `agents.change` are no longer supported. Use `agents.find` and `agents.solve`.

## Finding Shape
Generated findings are concise and structured:
- `Finding`: problem statement
- `Impact`: why it matters
- `Recommendation`: concrete fix direction
- `Next Step`: immediate first implementation action

Classification is encoded as issue labels (`severity:*` and `type:*`) instead of a metadata section in the issue body.

## Release (Maintainers)
Tagged releases (`v*.*.*`) trigger `.github/workflows/release-homebrew.yml`:
1. Runs `fmt`, `clippy`, and `test`.
2. Builds and publishes:
   - `thousand-darwin-arm64.tar.gz`
   - `thousand-darwin-x86_64.tar.gz`
   - `thousand-linux-x86_64.tar.gz`
3. Updates `Formula/thousand.rb` in `philipesteiff/homebrew-tap` using `HOMEBREW_TAP_TOKEN`.

CI checks run on every push commit and PR update via `.github/workflows/ci.yml` using:
- `just lint`
- `just test`

Local packaging smoke test:
```bash
just package
tar -tzf dist/thousand-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m | sed 's/aarch64/arm64/;s/amd64/x86_64/').tar.gz
```
