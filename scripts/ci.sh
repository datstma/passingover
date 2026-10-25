#!/usr/bin/env bash
# Run the project checks here and on remote Linux/Windows build hosts over SSH.
# Passing Over uses this instead of hosted CI (docs/decisions.md, D18).
#
# Usage: scripts/ci.sh [COMMAND]   (default: all)
#
#   local            Rust: fmt, clippy, tests, cargo-deny. Flutter (needs
#                    `flutter` on PATH or $FLUTTER): toolchain pins match,
#                    generated bindings are current, analyze, tests, and the
#                    integration test on this desktop
#   linux | windows  sync the working tree to that host, then Rust clippy and
#                    tests, and Flutter analyze, tests, a debug build and the
#                    integration test if the host has our Flutter SDK
#   all              local, then every configured remote
#   setup HOST       install the pinned Flutter SDK on HOST (linux|windows)
#   clean            remove sources and build output on every remote
#   purge            remove everything this script created on every remote
#
# Remote hosts come from the environment or from the git-ignored file
# .ci-hosts.local at the repo root, for example:
#
#   PASSINGOVER_CI_LINUX=user@linux-host
#   PASSINGOVER_CI_WINDOWS=user@windows-host   # OpenSSH, PowerShell 7 shell
#
# Build hosts may be shared with other projects. Everything this script
# creates on a remote lives in ~/passingover-ci: src/, target/, inbox/, and
# the pinned flutter/ SDK with its own pub-cache/. The pinned Rust toolchain
# is installed alongside the host's others; defaults are never changed.
set -euo pipefail

FLUTTER_VERSION=3.47.8
FLUTTER_ARCHIVES=https://storage.googleapis.com/flutter_infra_release/releases/stable
FLUTTER_LINUX_SHA256=baf7dcc5a3dbba60fe863bf493a6cf203195621f7786eaa5a1bba02cd0d87dd2
FLUTTER_WINDOWS_SHA256=11e19ea3409808fe3b3765ea8149640c299e58cce6eda7c9f38fc1b4a193eb6e

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

# --- local ------------------------------------------------------------------

run_local() {
  step "local: rust fmt"
  cargo fmt --all --check
  step "local: rust clippy"
  cargo $clippy_args
  step "local: rust test"
  cargo $test_args
  if cargo deny --version >/dev/null 2>&1; then
    step "local: cargo-deny"
    cargo deny --all-features check
  else
    echo "cargo-deny not installed; skipping dependency checks" >&2
  fi
  step "local: toolchain pins"
  check_pins
  run_local_flutter
}

check_pins() {
  local root app
  root="$(grep -E '^channel' rust-toolchain.toml)"
  app="$(grep -E '^channel' app/rust/rust-toolchain.toml)"
  if [[ "$root" != "$app" ]]; then
    echo "rust-toolchain.toml ($root) and app/rust/rust-toolchain.toml ($app) differ" >&2
    return 1
  fi
  echo "both pin ${root#*= }"
}

generated_checksum() {
  find app/lib/src/rust app/rust/src/frb_generated.rs -type f | sort | xargs cat | cksum
}

run_local_flutter() {
  local flutter="${FLUTTER:-$(command -v flutter || true)}"
  if [[ -z "$flutter" ]]; then
    echo "flutter not found (put it on PATH or set FLUTTER); skipping Flutter checks" >&2
    return 0
  fi
  step "local: generated bindings are current"
  if [[ -x .tools/bin/flutter_rust_bridge_codegen ]]; then
    local before after
    before="$(generated_checksum)"
    (cd app && ../.tools/bin/flutter_rust_bridge_codegen generate >/dev/null 2>&1)
    after="$(generated_checksum)"
    if [[ "$before" != "$after" ]]; then
      echo "The generated bindings were out of date and have been regenerated; review and commit them." >&2
      return 1
    fi
    echo "up to date"
  else
    echo "codegen not installed (scripts/install-tools.sh); skipping" >&2
  fi
  local device
  case "$(uname -s)" in
    Darwin) device=macos ;;
    Linux) device=linux ;;
    *) device=windows ;;
  esac
  (
    cd app
    "$flutter" pub get >/dev/null
    step "local: flutter analyze"
    "$flutter" analyze
    step "local: flutter test"
    "$flutter" test
    step "local: flutter integration test ($device, calls into Rust)"
    "$flutter" test integration_test -d "$device"
  )
}

# --- remotes ------------------------------------------------------------------

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

require_host() {
  [[ -n "$2" ]] || { echo "PASSINGOVER_CI_$1 not set; skipping" >&2; return 1; }
}

# Run a script (read from stdin) on a remote host. It is uploaded as a file
# and deleted afterwards: PowerShell misparses multi-line scripts read from
# stdin, and any command that reads stdin would swallow the rest of a script
# fed to `bash -s`. Scripts start with a preamble holding their settings.
run_remote_script() {
  local os="$1" host="$2" name="$3" file code=0
  file="$(mktemp -t passingover-ci.XXXXXX)"
  cat >"$file"
  if [[ "$os" == windows ]]; then
    ssh "$host" "New-Item -ItemType Directory -Force (Join-Path \$HOME '$remote_dir/inbox') | Out-Null"
    scp -q "$file" "$host:$remote_dir/inbox/$name.ps1"
    rm -f "$file"
    ssh "$host" "pwsh -NoProfile -NonInteractive -File (Join-Path \$HOME '$remote_dir/inbox/$name.ps1')" || code=$?
    ssh "$host" "Remove-Item -Force (Join-Path \$HOME '$remote_dir/inbox/$name.ps1') -ErrorAction SilentlyContinue"
  else
    ssh "$host" "mkdir -p ~/$remote_dir/inbox"
    scp -q "$file" "$host:$remote_dir/inbox/$name.sh"
    rm -f "$file"
    ssh "$host" "bash ~/$remote_dir/inbox/$name.sh </dev/null" || code=$?
    ssh "$host" "rm -f ~/$remote_dir/inbox/$name.sh"
  fi
  return "$code"
}

linux_preamble() {
  printf 'REMOTE_DIR=%q CLIPPY_ARGS=%q TEST_ARGS=%q FLUTTER_VERSION=%q\n' \
    "$remote_dir" "$clippy_args" "$test_args" "$FLUTTER_VERSION"
}

windows_preamble() {
  printf "\$RemoteDir = '%s'; \$ClippyArgs = '%s'; \$TestArgs = '%s'; \$FlutterVersion = '%s'\n" \
    "$remote_dir" "$clippy_args" "$test_args" "$FLUTTER_VERSION"
}

run_linux() {
  require_host LINUX "$linux_host" || return 0
  step "linux ($linux_host): sync"
  ssh "$linux_host" "mkdir -p ~/$remote_dir/inbox"
  upload "$linux_host"
  { linux_preamble; cat <<'SH'
set -euo pipefail
step() { printf '\n\033[1m==> linux: %s\033[0m\n' "$*"; }
root="$HOME/$REMOTE_DIR"
# Fresh sources; keep Flutter's build caches so Rust isn't rebuilt each run.
rm -rf "$root/src.new" && mkdir -p "$root/src.new"
tar -xf "$root/inbox/src.tar" -C "$root/src.new" && rm "$root/inbox/src.tar"
for d in app/.dart_tool app/build; do
  if [ -d "$root/src/$d" ]; then mv "$root/src/$d" "$root/src.new/$d"; fi
done
rm -rf "$root/src" && mv "$root/src.new" "$root/src"
cd "$root/src"
# Shared host: no debug info or incremental caches, to save disk.
export CARGO_TARGET_DIR="$root/target" CARGO_INCREMENTAL=0 \
  CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
step "rust clippy"
cargo $CLIPPY_ARGS
step "rust test"
cargo $TEST_ARGS
flutter="$root/flutter/bin/flutter"
if [ ! -x "$flutter" ]; then
  echo "No Flutter SDK in ~/$REMOTE_DIR (scripts/ci.sh setup linux); skipping Flutter."
  exit 0
fi
export PUB_CACHE="$root/pub-cache" FLUTTER_SUPPRESS_ANALYTICS=true
cd app
# In this CI copy only, list just the host's target, so rustup doesn't fetch
# the other eleven (about 1.4 GB) onto a shared machine.
tc="$(sed -n 's/^channel = "\(.*\)"/\1/p' rust/rust-toolchain.toml)"
host="$(rustc +"$tc" -vV | sed -n 's/^host: //p')"
awk -v t="$host" '/^targets = \[/ { print "targets = [\"" t "\"]"; skip = 1; next }
  skip && /^\]/ { skip = 0; next } !skip' rust/rust-toolchain.toml >rust/rust-toolchain.toml.ci
mv rust/rust-toolchain.toml.ci rust/rust-toolchain.toml
"$flutter" pub get >/dev/null
step "flutter analyze"
"$flutter" analyze
step "flutter test"
"$flutter" test
step "flutter build linux --debug"
"$flutter" build linux --debug
step "flutter integration test (linux, headless X, calls into Rust)"
xvfb-run -a "$flutter" test integration_test -d linux
SH
  } | run_remote_script linux "$linux_host" check
}

run_windows() {
  require_host WINDOWS "$windows_host" || return 0
  step "windows ($windows_host): sync"
  ssh "$windows_host" "New-Item -ItemType Directory -Force (Join-Path \$HOME '$remote_dir/inbox') | Out-Null"
  upload "$windows_host"
  { windows_preamble; cat <<'PS'
$ErrorActionPreference = 'Stop'
$PSStyle.OutputRendering = 'PlainText'
function Step($m) { Write-Host "`n==> windows: $m" }
function Check { if ($LASTEXITCODE) { exit $LASTEXITCODE } }
$root = Join-Path $HOME $RemoteDir
$src = Join-Path $root 'src'
$new = Join-Path $root 'src.new'
# Fresh sources; keep Flutter's build caches so Rust isn't rebuilt each run.
if (Test-Path $new) { Remove-Item -Recurse -Force $new }
New-Item -ItemType Directory $new | Out-Null
tar -xf (Join-Path $root 'inbox/src.tar') -C $new; Check
Remove-Item (Join-Path $root 'inbox/src.tar')
foreach ($d in 'app/.dart_tool', 'app/build') {
  if (Test-Path (Join-Path $src $d)) { Move-Item (Join-Path $src $d) (Join-Path $new $d) }
}
if (Test-Path $src) { Remove-Item -Recurse -Force $src }
Move-Item $new $src
Set-Location $src
# Shared host: no debug info or incremental caches, to save disk.
$env:CARGO_TARGET_DIR = Join-Path $root 'target'
$env:CARGO_INCREMENTAL = '0'; $env:CARGO_PROFILE_DEV_DEBUG = '0'; $env:CARGO_PROFILE_TEST_DEBUG = '0'
# rustup's proxies (cargo.exe, rustc.exe) may be symlinks that an elevated
# SSH session refuses to follow, so put the pinned toolchain's own bin
# directory first on PATH for this session only.
$tc = (Select-String -Path rust-toolchain.toml -Pattern '^channel\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
rustup toolchain install $tc --profile minimal --component clippy,rustfmt 2>&1 | Out-Null
$env:PATH = (Split-Path (rustup which --toolchain $tc cargo)) + ';' + $env:PATH
$env:RUSTUP_TOOLCHAIN = $tc
Step 'rust clippy'
cargo @($ClippyArgs -split ' '); Check
Step 'rust test'
cargo @($TestArgs -split ' '); Check
$flutter = Join-Path $root 'flutter/bin/flutter.bat'
if (-not (Test-Path $flutter)) {
  "No Flutter SDK in ~/$RemoteDir (scripts/ci.sh setup windows); skipping Flutter."
  exit 0
}
# The app's build hook picks its toolchain from app/rust/rust-toolchain.toml.
Remove-Item Env:RUSTUP_TOOLCHAIN
$env:PUB_CACHE = Join-Path $root 'pub-cache'; $env:FLUTTER_SUPPRESS_ANALYTICS = 'true'
Set-Location app
# In this CI copy only, list just the host's target, so rustup doesn't fetch
# the other eleven (about 1.4 GB) onto a shared machine.
$hostTriple = ((rustc -vV | Select-String '^host: ') -replace '^host: ', '').Trim()
$toolchainFile = Join-Path (Get-Location) 'rust/rust-toolchain.toml'
$toml = [IO.File]::ReadAllText($toolchainFile)
[IO.File]::WriteAllText($toolchainFile, ($toml -replace '(?s)targets = \[.*?\]', "targets = [`"$hostTriple`"]"))
& $flutter pub get | Out-Null; Check
Step 'flutter analyze'
& $flutter analyze; Check
Step 'flutter test'
& $flutter test; Check
Step 'flutter build windows --debug'
& $flutter build windows --debug; Check
Step 'flutter integration test (windows, calls into Rust)'
& $flutter test integration_test -d windows; $code = $LASTEXITCODE
Get-Process passingover -ErrorAction SilentlyContinue | Stop-Process -Force
exit $code
PS
  } | run_remote_script windows "$windows_host" check
}

run_setup() {
  case "${1:-}" in
    linux)
      require_host LINUX "$linux_host" || return 1
      step "linux ($linux_host): install Flutter $FLUTTER_VERSION into ~/$remote_dir/flutter"
      { linux_preamble; printf 'URL=%q SHA256=%q\n' \
          "$FLUTTER_ARCHIVES/linux/flutter_linux_${FLUTTER_VERSION}-stable.tar.xz" "$FLUTTER_LINUX_SHA256"
        cat <<'SH'
set -euo pipefail
root="$HOME/$REMOTE_DIR"
mkdir -p "$root/inbox"
missing=""
for p in clang cmake ninja-build pkg-config libgtk-3-dev xvfb; do
  dpkg -s "$p" >/dev/null 2>&1 || missing="$missing $p"
done
if [ -n "$missing" ]; then
  echo "Linux desktop builds also need these packages (not installed by this script):"
  echo "  sudo apt-get install -y$missing"
fi
if [ -x "$root/flutter/bin/flutter" ]; then echo "Flutter already installed"; exit 0; fi
archive="$root/inbox/flutter.tar.xz"
curl -sSfL -o "$archive" "$URL"
echo "$SHA256  $archive" | sha256sum -c --quiet
tar -xf "$archive" -C "$root" && rm "$archive"
PUB_CACHE="$root/pub-cache" FLUTTER_SUPPRESS_ANALYTICS=true "$root/flutter/bin/flutter" --version
SH
      } | run_remote_script linux "$linux_host" setup
      ;;
    windows)
      require_host WINDOWS "$windows_host" || return 1
      step "windows ($windows_host): install Flutter $FLUTTER_VERSION into ~/$remote_dir/flutter"
      { windows_preamble
        printf "\$Url = '%s'; \$Sha256 = '%s'\n" \
          "$FLUTTER_ARCHIVES/windows/flutter_windows_${FLUTTER_VERSION}-stable.zip" "$FLUTTER_WINDOWS_SHA256"
        cat <<'PS'
$ErrorActionPreference = 'Stop'
$PSStyle.OutputRendering = 'PlainText'
$root = Join-Path $HOME $RemoteDir
New-Item -ItemType Directory -Force (Join-Path $root 'inbox') | Out-Null
if (Test-Path (Join-Path $root 'flutter/bin/flutter.bat')) { 'Flutter already installed'; exit 0 }
$zip = Join-Path $root 'inbox/flutter.zip'
curl.exe -sSfL -o $zip $Url
if ((Get-FileHash -Algorithm SHA256 $zip).Hash.ToLower() -ne $Sha256) { Remove-Item $zip; throw 'checksum mismatch' }
tar -xf $zip -C $root
if ($LASTEXITCODE) { exit $LASTEXITCODE }
Remove-Item $zip
$env:PUB_CACHE = Join-Path $root 'pub-cache'; $env:FLUTTER_SUPPRESS_ANALYTICS = 'true'
& (Join-Path $root 'flutter/bin/flutter.bat') --version
PS
      } | run_remote_script windows "$windows_host" setup
      ;;
    *)
      echo "usage: scripts/ci.sh setup linux|windows" >&2
      return 2
      ;;
  esac
}

# Remove build output only (keep: flutter/, pub-cache/), or everything.
run_clean() {
  local what="$1"
  if [[ -n "$linux_host" ]]; then
    step "linux ($linux_host): $what ~/$remote_dir"
    if [[ "$what" == purge ]]; then
      ssh "$linux_host" "rm -rf ~/$remote_dir"
    else
      ssh "$linux_host" "rm -rf ~/$remote_dir/src ~/$remote_dir/src.new ~/$remote_dir/target ~/$remote_dir/inbox"
    fi
  fi
  if [[ -n "$windows_host" ]]; then
    step "windows ($windows_host): $what ~/$remote_dir"
    if [[ "$what" == purge ]]; then
      ssh "$windows_host" "\$d = Join-Path \$HOME '$remote_dir'; if (Test-Path \$d) { Remove-Item -Recurse -Force \$d }"
    else
      ssh "$windows_host" "foreach (\$n in 'src','src.new','target','inbox') { \$d = Join-Path \$HOME '$remote_dir' | Join-Path -ChildPath \$n; if (Test-Path \$d) { Remove-Item -Recurse -Force \$d } }"
    fi
  fi
}

case "${1:-all}" in
  local) run_local ;;
  linux) run_linux ;;
  windows) run_windows ;;
  all) run_local; run_linux; run_windows ;;
  setup) run_setup "${2:-}" ;;
  clean) run_clean clean ;;
  purge) run_clean purge ;;
  *) sed -n '2,28p' "$0" >&2; exit 2 ;;
esac

step "done"
