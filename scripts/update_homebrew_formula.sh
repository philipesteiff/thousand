#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

BINARY_NAME="${BINARY_NAME:-thousand}"
FORMULA_NAME="${FORMULA_NAME:-thousand}"
TAP_DIR="${TAP_DIR:-/Users/philipesteiff/Projects/homebrew-tap}"
ASSET_DIR="${ASSET_DIR:-dist}"
FORMULA_CLASS="${FORMULA_CLASS:-$(echo "${FORMULA_NAME}" | awk -F'[-_]' '{for (i=1;i<=NF;i++){printf toupper(substr($i,1,1)) substr($i,2)} }')}"

VERSION="${VERSION:-}"
TAG="${TAG:-}"
REPO_SLUG="${REPO_SLUG:-${GITHUB_REPOSITORY:-}}"

if [[ -z "${VERSION}" ]]; then
  if [[ -n "${TAG}" ]]; then
    VERSION="${TAG#v}"
  else
    echo "Missing VERSION (or TAG)." >&2
    exit 1
  fi
fi

if [[ -z "${TAG}" ]]; then
  TAG="v${VERSION}"
fi

if [[ -z "${REPO_SLUG}" ]]; then
  ORIGIN_URL="$(git remote get-url origin 2>/dev/null || true)"
  case "${ORIGIN_URL}" in
    git@github.com:*)
      REPO_SLUG="${ORIGIN_URL#git@github.com:}"
      REPO_SLUG="${REPO_SLUG%.git}"
      ;;
    https://github.com/*)
      REPO_SLUG="${ORIGIN_URL#https://github.com/}"
      REPO_SLUG="${REPO_SLUG%.git}"
      ;;
  esac
fi

if [[ -z "${REPO_SLUG}" ]]; then
  echo "Missing REPO_SLUG (or GITHUB_REPOSITORY/origin remote)." >&2
  exit 1
fi

DARWIN_ARM64_ASSET="${BINARY_NAME}-darwin-arm64.tar.gz"
DARWIN_X86_64_ASSET="${BINARY_NAME}-darwin-x86_64.tar.gz"
LINUX_X86_64_ASSET="${BINARY_NAME}-linux-x86_64.tar.gz"

read_or_compute_sha() {
  local existing_sha="$1"
  local asset_file="$2"

  if [[ -n "${existing_sha}" ]]; then
    printf '%s\n' "${existing_sha}"
    return 0
  fi

  if [[ ! -f "${ASSET_DIR}/${asset_file}" ]]; then
    echo "Missing ${ASSET_DIR}/${asset_file} and no SHA provided." >&2
    exit 1
  fi

  shasum -a 256 "${ASSET_DIR}/${asset_file}" | awk '{print $1}'
}

DARWIN_ARM64_SHA="$(read_or_compute_sha "${DARWIN_ARM64_SHA:-}" "${DARWIN_ARM64_ASSET}")"
DARWIN_X86_64_SHA="$(read_or_compute_sha "${DARWIN_X86_64_SHA:-}" "${DARWIN_X86_64_ASSET}")"
LINUX_X86_64_SHA="$(read_or_compute_sha "${LINUX_X86_64_SHA:-}" "${LINUX_X86_64_ASSET}")"

DARWIN_ARM64_URL="https://github.com/${REPO_SLUG}/releases/download/${TAG}/${DARWIN_ARM64_ASSET}"
DARWIN_X86_64_URL="https://github.com/${REPO_SLUG}/releases/download/${TAG}/${DARWIN_X86_64_ASSET}"
LINUX_X86_64_URL="https://github.com/${REPO_SLUG}/releases/download/${TAG}/${LINUX_X86_64_ASSET}"

FORMULA_DIR="${TAP_DIR}/Formula"
FORMULA_PATH="${FORMULA_DIR}/${FORMULA_NAME}.rb"

mkdir -p "${FORMULA_DIR}"
cat <<RUBY | sed 's/^  //' > "${FORMULA_PATH}"
  class ${FORMULA_CLASS} < Formula
    desc "Autonomous repository improvement orchestrator"
    homepage "https://github.com/${REPO_SLUG}"
    version "${VERSION}"

    on_macos do
      on_arm do
        url "${DARWIN_ARM64_URL}"
        sha256 "${DARWIN_ARM64_SHA}"
      end

      on_intel do
        url "${DARWIN_X86_64_URL}"
        sha256 "${DARWIN_X86_64_SHA}"
      end
    end

    on_linux do
      on_intel do
        url "${LINUX_X86_64_URL}"
        sha256 "${LINUX_X86_64_SHA}"
      end
    end

    def install
      bin.install "${BINARY_NAME}"
    end

    test do
      assert_match version.to_s, shell_output("#{bin}/${BINARY_NAME} version")
    end
  end
RUBY

echo "Wrote ${FORMULA_PATH}"
