# Common commands for this repo. Run with `just <task>`.

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# List available tasks.
default:
  @just --list

# Build the CLI in debug mode.
build:
  cargo build

# Run tests.
test:
  cargo test

# Format code.
fmt:
  cargo fmt

# Run clippy lints.
clippy:
  cargo clippy --all-targets --all-features -- -D warnings

# Format check + lint.
lint:
  cargo fmt --check
  cargo clippy --all-targets --all-features -- -D warnings

# Build and package a release archive for the current host.
package:
  bash scripts/package.sh

# Update Formula/thousand.rb in the local Homebrew tap.
update-homebrew-formula:
  bash scripts/update_homebrew_formula.sh
