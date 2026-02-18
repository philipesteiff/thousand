# Thousand CLI

`thousand` orchestrates two explicit non-interactive phases for autonomous repo improvements:

1. `find`: identify one finding and open a GitHub issue.
2. `solve`: implement one labeled finding issue and open a PR.

## Prerequisites
- `codex` CLI installed on `PATH`
- `git` configured with `origin` in the target repo
- GitHub API token: `THOUSAND_GH_TOKEN` (fallback: `GITHUB_TOKEN`)
- CI Codex auth: `CODEX_API_KEY` (recommended for non-interactive/headless runs)

## Auth Modes
- Local interactive auth: use `codex login`; `thousand` will use the existing Codex login session.
- CI/headless auth: set `CODEX_API_KEY`; `thousand` runs `codex exec` and inherits the process environment.
- `thousand` does not override Codex auth mode; it relies on Codex CLI authentication behavior.

## CI Example (API Key + GitHub Token)
```yaml
jobs:
  thousand:
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4
      - name: Run thousand
        env:
          CODEX_API_KEY: ${{ secrets.CODEX_API_KEY }}
          THOUSAND_GH_TOKEN: ${{ secrets.THOUSAND_GH_TOKEN }}
        run: |
          thousand validate --config .thousand/workflow.yaml
          thousand find --config .thousand/workflow.yaml
          # after selecting an issue to implement:
          thousand solve --config .thousand/workflow.yaml --issue 7
```

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
1. Runs `just lint` and `just test` on both `ubuntu-22.04` and `macos-14`.
2. Builds and publishes:
   - `thousand-darwin-arm64.tar.gz`
   - `thousand-darwin-x86_64.tar.gz`
   - `thousand-linux-x86_64.tar.gz`
3. (Optional strict mode) validates pushed tag matches `Cargo.toml` version when repo variable `STRICT_RELEASE=true`.
4. Updates `Formula/thousand.rb` via `scripts/update_homebrew_formula.sh`.
   - CI uses `TAP_DIR=homebrew-tap` after checking out `philipesteiff/homebrew-tap`.
   - Formula uses Homebrew platform/arch blocks (`on_macos`, `on_linux`, `on_arm`, `on_intel`) for per-platform assets.
   - Local default is `TAP_DIR=/Users/philipesteiff/Projects/homebrew-tap`.
5. Audits the generated formula with `brew audit --strict`.

Local formula update:
```bash
VERSION=0.1.0 TAG=v0.1.0 REPO_SLUG=philipesteiff/thousand just update-homebrew-formula
test -f /Users/philipesteiff/Projects/homebrew-tap/Formula/thousand.rb
```
If `dist/` is missing any release archive, pass the missing checksum(s) explicitly:
```bash
DARWIN_X86_64_SHA=<sha256> just update-homebrew-formula
```

CI checks run on every push commit and PR update via `.github/workflows/ci.yml` using:
- `just lint`
- `just test`

Local packaging smoke test:
```bash
just package
tar -tzf dist/thousand-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m | sed 's/aarch64/arm64/;s/amd64/x86_64/').tar.gz
```
