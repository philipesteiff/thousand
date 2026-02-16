#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

BINARY_NAME="thousand"
TARGET_OS="${TARGET_OS:-$(uname -s | tr '[:upper:]' '[:lower:]')}"
TARGET_ARCH="${TARGET_ARCH:-$(uname -m)}"
RUST_TARGET="${RUST_TARGET:-}"

case "${TARGET_ARCH}" in
  aarch64 | arm64) TARGET_ARCH="arm64" ;;
  x86_64 | amd64) TARGET_ARCH="x86_64" ;;
  *)
    echo "Unsupported architecture: ${TARGET_ARCH}" >&2
    exit 1
    ;;
esac

if [[ -n "${RUST_TARGET}" ]]; then
  cargo build --release --target "${RUST_TARGET}"
  BUILD_DIR="target/${RUST_TARGET}/release"
else
  cargo build --release
  BUILD_DIR="target/release"
fi

if [[ ! -f "${BUILD_DIR}/${BINARY_NAME}" ]]; then
  echo "Missing ${BUILD_DIR}/${BINARY_NAME}. Did the build succeed?" >&2
  exit 1
fi

mkdir -p dist
ARCHIVE="dist/${BINARY_NAME}-${TARGET_OS}-${TARGET_ARCH}.tar.gz"
tar -czf "${ARCHIVE}" -C "${BUILD_DIR}" "${BINARY_NAME}"

echo "Wrote ${ARCHIVE}"
