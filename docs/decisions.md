# Decision log

Every non-trivial choice gets a numbered entry. Numbers are never reused or
renumbered. To reverse a decision, add a new entry that supersedes it and mark
the old one *Superseded by Dn*.

---

### D1 — Name and identifiers
*2026-10-25 · Accepted (seed brief)*

- Display name **Passing Over**, tagline *Where the planes above you are going.*
- Handle `passingover` everywhere else: GitHub repo `datstma/passingover`,
  crate prefixes (`passingover-core`, `passingover-cli`, …), package names,
  CLI binary.
- Bundle / application identifier `se.mansby.passingover`; Dart package
  `passingover`.

**Why:** one short handle that is valid as a crate, a Dart package and a command.

### D2 — License: MIT OR Apache-2.0
*2026-10-25 · Accepted (seed brief)*

Dual-licensed. Copyright line: "Stefan Månsby and the Passing Over
contributors". Contributions are dual-licensed by default (Apache-2.0 §5
wording in README and CONTRIBUTING).

**Why:** Rust-ecosystem convention; Apache-2.0 adds a patent grant, MIT keeps
reuse simple.

### D3 — Stack: Flutter UI, Rust core, flutter_rust_bridge v2
*2026-10-25 · Accepted (seed brief)*

One Flutter codebase for macOS, Windows, Linux, iOS and Android. All logic
lives in a Rust crate with no Flutter dependency, bridged with
flutter_rust_bridge v2.

**Why:** one UI codebase for five platforms; the core stays testable and
reusable without Flutter (CLI, tests, a future web/WASM build).

### D4 — Toolchain versions at project start
*2026-10-25 · Accepted*

| Tool | Version | Released | Source |
|---|---|---|---|
| Flutter (stable) | 3.47.8, Dart 3.13.7 | 2026-10-23 | Flutter releases JSON |
| Rust (stable) | 1.99.1 | 2026-10-13 | `rustup check` |
| flutter_rust_bridge (crate, codegen, Dart package) | 2.13.0 | 2026-08-23 | crates.io, pub.dev |
| clap | 4.6 | 2026-09-14 | crates.io |

2.14.0 of flutter_rust_bridge is in beta (2.14.0-beta.2) and is not used.

**Why:** latest stable of each when the project started.

### D5 — Pin the Rust toolchain exactly
*2026-10-25 · Accepted*

`rust-toolchain.toml` at the repo root and in `app/rust/` both pin `1.99.1`;
the workspace declares `rust-version = "1.99"`. CI fails if the two files
disagree. Bumps happen deliberately in their own commit, with a new entry here.

**Why:** the Native Assets build backend (D7) refuses floating channels such as
`stable`. Pinning everywhere keeps local, CI and app builds on one compiler.
Passing Over is an app, not a library, so its MSRV can be the pinned version.

### D6 — Rust workspace conventions
*2026-10-25 · Accepted*

Edition 2024, resolver 3, shared `[workspace.package]` metadata and workspace
lints (`unsafe_code = "forbid"`, `clippy::all` at warn; CI runs clippy with
`-D warnings`). The bridge crate does not inherit `unsafe_code = "forbid"`
because the generated FFI glue needs `unsafe`.

**Why:** consistent metadata and lint posture; `unsafe` stays confined to
generated bridge code.

### D7 — Build Rust into the app with frb's Native Assets backend, not Cargokit
*2026-10-25 · Accepted*

`flutter_rust_bridge_codegen integrate --integration-backend native-assets`.
Flutter builds the Rust crate through a Dart build hook (`app/hook/build.dart`
→ `flutter_rust_bridge_hooks` → `native_toolchain_rust`).

**Why:** frb's default Cargokit backend ships a CocoaPods-only podspec (no
`Package.swift`), while Flutter ≥ 3.44 defaults to Swift Package Manager and
the CocoaPods trunk becomes read-only on 2026-12-02. Native Assets needs no
CocoaPods and no `rust_builder` plugin package, and frb describes it as
"possibly the preferred way in the future". **Fallback:** the Cargokit backend
(frb documents the migration both ways). The Rust and Dart APIs are the same
under either backend.

### D8 — Repository layout adjustments
*2026-10-25 · Accepted*

The layout follows the seed brief, with these additions:

- `app/hook/build.dart`: the build hook required by D7.
- No `app/rust_builder/`; that directory only exists with Cargokit.
- `app/rust/` is a member of the root Cargo workspace.
- `app/rust/rust-toolchain.toml`: `native_toolchain_rust` reads it from the
  crate directory and requires an exact version plus a target list.
- `.github/workflows/` for CI.

**Why:** one `Cargo.lock`, and `cargo test` / `cargo clippy --workspace` cover
the bridge crate as well.

### D9 — CLI built with clap
*2026-10-25 · Accepted*

The `passingover` binary lives in `crates/passingover-cli` and uses clap's
derive API. M0 has `hello` and `--version`; M1 adds `now --lat --lon`.

**Why:** clap is the de-facto standard and keeps subcommands declarative.

### D10 — Live positions: adsb.lol primary, adsb.fi fallback
*2026-10-25 · Accepted for development; confirm with operators before release*

`PositionSource` implementations for adsb.lol (`/v2/point/{lat}/{lon}/{nm}`)
and adsb.fi (`/api/v3/lat/…/lon/…/dist/…`). Both return the readsb v2 JSON
shape, so they share one parser. Not used: OpenSky (terms appear to require a
written agreement for a live product; 400 anonymous credits/day),
airplanes.live (HTTP 403 to anonymous requests), ADS-B Exchange (paid, needs a
key).

**Why:** keyless, global, same format; adsb.lol's data is ODbL, which permits
commercial use. Open risks: adsb.lol sends a `500;w=86400` rate-limit header
and plans a key; adsb.fi is personal/non-commercial only. Both are listed in
docs/data-sources.md, with the operators to contact before release.

### D11 — Routes: bundled VRS standing data, adsbdb online fallback
*2026-10-25 · Accepted*

Callsign → route comes from a bundled snapshot of the Virtual Radar Server
standing data (CC0, updated daily). For callsigns it lacks, adsbdb is queried
live, and its results are cached in memory only, because its route data may
not be stored or redistributed. Every route must pass a plausibility check
(aircraft near the origin→destination great circle and heading toward the
destination) or it is shown as unknown.

**Why:** public-domain, offline, no per-user requests and no location leaves
the device; crowd-sourced routes are sometimes wrong, so they get checked.

### D12 — Time to landing is an estimate
*2026-10-25 · Accepted*

No free, keyless source gives scheduled or estimated arrival times. "Landing
in …" = great-circle distance remaining ÷ ground speed (a typical cruise speed
while climbing), plus ~15 min for the approach. It is shown as approximate and
rounded to 5 min.

**Why:** keyed APIs need a proxy and have quotas far too small for per-user calls.

### D13 — Weather: Open-Meteo primary, MET Norway fallback
*2026-10-25 · Accepted (non-commercial terms are fine per D16)*

Weather is fetched for the **destination airport's** coordinates (never the
user's). Open-Meteo's free API is the primary source and also returns the IANA
time zone; MET Norway Locationforecast is the fallback (identifying
User-Agent, honour `Expires`). Both need on-screen attribution.

**Why:** keyless, global and simple. Open-Meteo's free tier is non-commercial
only, which fits D16: Open-Meteo counts apps without subscriptions or
advertising as non-commercial.

### D14 — Airports and time zones bundled; tz computed at build time
*2026-10-25 · Accepted*

The airport table comes from VRS airports (CC0; guaranteed to match VRS route
codes), with gaps filled from OurAirports (public domain). Neither has a time
zone, so a build step uses `tzf-rs` to add an IANA `tz` column, and the app
uses `chrono-tz` to get the current offset. mwgg/Airports (MIT) is a
cross-check. The generated table is published as a separate data file under
the ODbL with attribution, because the tzf boundary data is ODbL. The code
license is unaffected.

**Why:** fully offline lookups, no per-user requests, and the license
obligations are kept clean and explicit.

### D15 — Politeness and privacy defaults
*2026-10-25 · Accepted*

- Positions every 15 s while visible (never under 10 s, ±2 s jitter); paused
  when hidden. Ambient mode polls at 60 s until a source agrees to more.
- Default radius 20 nm (~37 km), user range 5–25 nm; one request per poll.
- User coordinates rounded to 2 decimals (~1.1 km) before any request; route
  and weather requests carry only callsigns or destination coordinates.
- User-Agent `PassingOver/<version> (+https://github.com/datstma/passingover)`.
- Backoff: honour `Retry-After` and rate-limit headers; exponential backoff
  with jitter capped at 10 min; switch to the fallback after 3 failures; never
  retry 401/403 in a loop.
- Cache TTLs, timeouts and the attribution screen are as in
  docs/data-sources.md ("Politeness defaults").

**Why:** keeps a distributed, keyless app within the community sources' limits
and keeps the user's location coarse.

### D16 — Pure open source, no paid tier
*2026-10-25 · Accepted (owner)*

Passing Over stays free and open source: no paid tier, subscriptions, ads or
commercial offering.

**Why:** the owner's intent. It also means the non-commercial terms of
community data sources (adsb.fi, Open-Meteo's free API) fit the project,
which removes the "commercial path" risk from docs/data-sources.md.
