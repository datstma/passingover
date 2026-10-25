#!/usr/bin/env bash
# Run the project checks here and on remote Linux/Windows build hosts over SSH.
# Passing Over uses this instead of hosted CI (docs/decisions.md, D18).
#
# Usage: scripts/ci.sh [all|local|linux|windows|clean]   (default: all)
#
#   local    fmt, clippy, tests and cargo-deny on this machine
#   linux    sync the working tree to the Linux host, run clippy and tests
#   windows  the same on the Windows host
#   all      local, then every configured remote
#   clean    delete the project directory on every configured remote
#
# Remote hosts come from the environment or from the git-ignored file
# .ci-hosts.local at the repo root, for example:
#
#   PASSINGOVER_CI_LINUX=user@linux-host
#   PASSINGOVER_CI_WINDOWS=user@windows-host   # OpenSSH, PowerShell 7 shell
#
# Build hosts may be shared with other projects. Everything this script
# creates on a remote lives in ~/passingover-ci (src/, target/, inbox/);
# `clean` removes it. The pinned Rust toolchain is installed alongside the
# host's others by rustup; defaults are never changed.
set -euo pipefail

repo="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo"

if [[ -f .ci-hosts.local ]]; then
  # shellcheck source=/dev/null
  source .ci-hosts.local
fi
linux_host="${PASSINGOVER_CI_LINUX:-}"
windows_host="${PASSINGOVER_CI_WINDOWS:-}"

remote_dir="passingover-ci"
clippy_args="clippy --workspace --all-targets --locked -- -D warnings"
test_args="test --workspace --locked"

step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

run_local() {
  step "local: fmt"
  cargo fmt --all --check
  step "local: clippy"
  cargo $clippy_args
  step "local: test"
  cargo $test_args
  if cargo deny --version >/dev/null 2>&1; then
    step "local: cargo-deny"
    cargo deny --all-features check
  else
    echo "cargo-deny not installed; skipping dependency checks" >&2
  fi
}

# Tar the working tree as git sees it: tracked files plus untracked files
# that aren't ignored, skipping tracked files deleted in the working tree.
make_tarball() {
  git ls-files -z --cached --others --exclude-standard \
    | while IFS= read -r -d '' f; do [[ -e "$f" ]] && printf '%s\0' "$f"; done \
    | COPYFILE_DISABLE=1 tar -cf "$1" --no-xattrs --null -T -
}

upload() {
  local host="$1" tarball
  tarball="$(mktemp -t passingover-ci.XXXXXX)"
  make_tarball "$tarball"
  scp -q "$tarball" "$host:$remote_dir/inbox/src.tar"
  rm -f "$tarball"
}

run_linux() {
  [[ -n "$linux_host" ]] || { echo "PASSINGOVER_CI_LINUX not set; skipping Linux" >&2; return 0; }
  step "linux ($linux_host): sync"
  ssh "$linux_host" "mkdir -p ~/$remote_dir/inbox"
  upload "$linux_host"
  step "linux ($linux_host): clippy and test"
  ssh "$linux_host" "set -e
    root=~/$remote_dir
    rm -rf \"\$root/src\" && mkdir \"\$root/src\"
    tar -xf \"\$root/inbox/src.tar\" -C \"\$root/src\" && rm \"\$root/inbox/src.tar\"
    cd \"\$root/src\"
    export CARGO_TARGET_DIR=\"\$root/target\"
    # Shared hosts: no debug info or incremental caches, to save disk.
    export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
    cargo $clippy_args
    cargo $test_args"
}

run_windows() {
  [[ -n "$windows_host" ]] || { echo "PASSINGOVER_CI_WINDOWS not set; skipping Windows" >&2; return 0; }
  step "windows ($windows_host): sync"
  ssh "$windows_host" "New-Item -ItemType Directory -Force (Join-Path \$HOME '$remote_dir/inbox') | Out-Null"
  upload "$windows_host"
  step "windows ($windows_host): clippy and test"
  ssh "$windows_host" "\$ErrorActionPreference = 'Stop'
    \$root = Join-Path \$HOME '$remote_dir'
    \$src = Join-Path \$root 'src'
    if (Test-Path \$src) { Remove-Item -Recurse -Force \$src }
    New-Item -ItemType Directory \$src | Out-Null
    tar -xf (Join-Path \$root 'inbox/src.tar') -C \$src
    if (\$LASTEXITCODE) { exit \$LASTEXITCODE }
    Remove-Item (Join-Path \$root 'inbox/src.tar')
    Set-Location \$src
    \$env:CARGO_TARGET_DIR = Join-Path \$root 'target'
    # Shared hosts: no debug info or incremental caches, to save disk.
    \$env:CARGO_INCREMENTAL = '0'; \$env:CARGO_PROFILE_DEV_DEBUG = '0'; \$env:CARGO_PROFILE_TEST_DEBUG = '0'
    # rustup's proxies (cargo.exe, rustc.exe) may be symlinks that an elevated
    # SSH session refuses to follow, so put the pinned toolchain's own bin
    # directory first on PATH for this session only.
    \$tc = (Select-String -Path rust-toolchain.toml -Pattern '^channel\s*=\s*\"([^\"]+)\"').Matches[0].Groups[1].Value
    rustup toolchain install \$tc --profile minimal --component clippy,rustfmt 2>&1 | Out-Null
    \$env:PATH = (Split-Path (rustup which --toolchain \$tc cargo)) + ';' + \$env:PATH
    \$env:RUSTUP_TOOLCHAIN = \$tc
    cargo $clippy_args
    if (\$LASTEXITCODE) { exit \$LASTEXITCODE }
    cargo $test_args
    exit \$LASTEXITCODE"
}

run_clean() {
  if [[ -n "$linux_host" ]]; then
    step "linux ($linux_host): remove ~/$remote_dir"
    ssh "$linux_host" "rm -rf ~/$remote_dir"
  fi
  if [[ -n "$windows_host" ]]; then
    step "windows ($windows_host): remove ~/$remote_dir"
    ssh "$windows_host" "\$d = Join-Path \$HOME '$remote_dir'; if (Test-Path \$d) { Remove-Item -Recurse -Force \$d }"
  fi
}

case "${1:-all}" in
  local) run_local ;;
  linux) run_linux ;;
  windows) run_windows ;;
  all) run_local; run_linux; run_windows ;;
  clean) run_clean ;;
  *) sed -n '2,12p' "$0" >&2; exit 2 ;;
esac

step "done"
