# Passing Over — project brief

*Where the planes above you are going.*

This is the durable brief for anyone (human or Claude) working in this repo.
Read it first; details live in `docs/`.

## Vision

Passing Over shows the aircraft overhead right now, built for daydreaming
rather than plane-spotting. Other apps answer "what is that?" with callsigns
and altitudes. Passing Over answers "where is that going, and what's it like
there?":

- "SK 945 → Chicago. It's 15:12 there, 9°C and overcast. Landing in 7h 40m."
- The journey as a story: where it left from, how long ago, what it will cross.
- A calm ambient mode for a desktop, a spare tablet or a TV (a free, software
  FlightWall).
- Later, a "daydream log" ("this month, planes over your home went to 61 cities").

## Product principles

- Calm over busy. Few numbers, beautiful typography and motion.
- Nothing that feels like an aviation instrument unless the user opts into a
  "technical" layout.
- Metric by default, imperial available.
- Privacy: the user's location never leaves the device except as a query to
  the data sources, and coordinates are rounded to ~1 km first.

## Naming

| What | Value |
|---|---|
| Display name | Passing Over |
| Handle (repo, crates, packages, CLI) | `passingover` |
| GitHub | `datstma/passingover` (owner creates it; never create or push a remote) |
| Bundle / application ID | `se.mansby.passingover` |
| Dart package | `passingover` |
| Crates | `passingover-core`, `passingover-cli`, bridge crate in `app/rust` |

## Stack

- **UI:** Flutter 3.47.8 (Dart 3.13.7), one codebase for macOS, Windows, Linux,
  iOS and Android.
- **Core:** Rust 1.99.1 (pinned in `rust-toolchain.toml`), edition 2024. The
  core has no Flutter dependency.
- **Bridge:** flutter_rust_bridge 2.13.0, Native Assets backend (Rust is built
  by `app/hook/build.dart`; no CocoaPods, no Cargokit). See D7.
- **License:** MIT OR Apache-2.0.

## Layout

```
passingover/
├── CLAUDE.md, README.md, CONTRIBUTING.md, LICENSE-MIT, LICENSE-APACHE
├── Cargo.toml              # Rust workspace (core, cli, app/rust)
├── rust-toolchain.toml     # pinned toolchain — keep in sync with app/rust/
├── crates/
│   ├── passingover-core/   # pure Rust: sources, filtering, enrichment, stories
│   └── passingover-cli/    # `passingover` binary
├── app/                    # Flutter app (all five platforms) — NOT YET CREATED, see M0
│   ├── hook/build.dart     # builds app/rust via native_toolchain_rust
│   ├── rust/               # thin flutter_rust_bridge crate → passingover-core
│   ├── lib/src/rust/       # GENERATED Dart bindings — do not edit
│   └── integration_test/   # runs the app and calls into Rust
├── design/                 # design system (placeholder)
├── docs/
│   ├── decisions.md        # numbered decision log (D1, D2, …)
│   ├── milestones.md       # M0…M7 with status
│   └── data-sources.md     # every external source: terms, limits, attribution
└── .github/workflows/      # CI
```

## Architecture rules

- **All logic goes in `passingover-core`.** The bridge crate (`app/rust`) only
  converts types and forwards calls. If something is useful to the CLI too, it
  belongs in core.
- Data sources are pluggable behind traits (`PositionSource`, `RouteSource`,
  `WeatherSource`, …). v1 needs no API keys.
- Be polite to the sources: poll every ~10–15 s, keep the default radius small,
  cache routes, airports and weather, and back off on errors. Send an
  identifying User-Agent. Concrete numbers are in `docs/data-sources.md`.
- Round coordinates to ~1 km before any request leaves the device.
- Location input: device location in the app; manual lat/lon everywhere;
  "pick on map" later.
- Never add a data source without documenting its terms, limits and
  attribution in `docs/data-sources.md` and logging a decision.

## Commands

Rust (from the repo root):

```sh
cargo build --workspace
cargo test --workspace                                  # fixtures only, no network
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cargo run -p passingover-cli -- hello
```

Flutter (from `app/`):

```sh
flutter pub get
flutter run -d macos                       # builds the Rust crate via the hook
flutter analyze
flutter test                               # Dart unit/widget tests
flutter test integration_test -d macos     # launches the app, calls into Rust
```

After changing the bridge API in `app/rust/src/api/`:

```sh
cd app && flutter_rust_bridge_codegen generate
```

The codegen tool is installed with
`cargo install flutter_rust_bridge_codegen --version 2.13.0 --locked`. Its
version must match the `flutter_rust_bridge` crate and Dart package.

## Testing rules

- Unit tests use recorded JSON fixtures; no live network in tests.
- One opt-in integration test may hit live sources. It is marked `#[ignore]`
  and runs with `cargo test -- --ignored`.

## Working agreements

- Log every non-trivial choice in `docs/decisions.md` as the next Dn, with a
  one-line rationale. Update `docs/milestones.md` status as work lands.
- Small, reviewable commits with clear messages, on `main`. No remote: never
  create one or push.
- Ask the owner before anything hard to undo, or before installing system-wide
  tooling beyond the usual Rust/Flutter toolchains. If a prerequisite is
  missing (Flutter SDK, Xcode, Android SDK, Rust targets), say exactly what to
  install rather than working around it.
- Toolchain bumps: change `rust-toolchain.toml` and
  `app/rust/rust-toolchain.toml` together (CI checks this once `app/` exists),
  and log a decision.
- Generated files (`app/lib/src/rust/**`, `app/rust/src/frb_generated.rs`) are
  regenerated, never hand-edited.
