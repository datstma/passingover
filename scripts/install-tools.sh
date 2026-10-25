#!/usr/bin/env bash
# Install the project's pinned developer tools into .tools/ (git-ignored), so
# they never collide with other projects' versions in ~/.cargo/bin.
#
# Usage: scripts/install-tools.sh
set -euo pipefail

repo="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo"

# Must match the flutter_rust_bridge crate and Dart package versions.
FRB_VERSION=2.13.0

cargo install --locked --root "$repo/.tools" \
  flutter_rust_bridge_codegen --version "$FRB_VERSION"

echo "Tools installed in $repo/.tools/bin"
