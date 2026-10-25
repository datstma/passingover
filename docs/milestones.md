# Milestones

Status values: **Not started** · **In progress** · **Done**.

| # | Milestone | Status |
|---|---|---|
| M0 | Foundation | In progress |
| M1 | Core + CLI | Done |
| M2 | First app view | Not started |
| M3 | Daydream enrichment | Not started |
| M4 | Ambient mode | Not started |
| M5 | Daydream log | Not started |
| M6 | Native extras | Not started |
| M7 | Release | Not started |

---

## M0 — Foundation

Docs written; the Rust workspace builds; the Flutter app exists with
flutter_rust_bridge wired up; a "hello from Rust core" call works on macOS
desktop; CI builds and tests the Rust workspace and the Flutter app on macOS,
Linux and Windows (local CI on the owner's machines, D18).

- [x] Git repository on `main`, published at github.com/datstma/passingover
- [x] Licenses (MIT, Apache-2.0)
- [x] Rust workspace: `passingover-core`, `passingover-cli` (`passingover hello`)
- [x] Docs: CLAUDE.md, README, CONTRIBUTING, decisions, milestones, design placeholder
- [x] docs/data-sources.md: first research pass; source decisions D10–D15 (D10 since superseded by D17)
- [x] CI: `scripts/ci.sh` runs fmt, clippy `-D warnings`, tests and cargo-deny
      on this Mac and clippy + tests on the Linux and Windows hosts (D18, D19)
- [x] Repo hygiene: issue/PR templates, SECURITY.md, code of conduct,
      .editorconfig, .gitattributes
- [x] `flutter_rust_bridge_codegen` 2.13.0 in `.tools/` via `scripts/install-tools.sh` (D20)
- [ ] **Blocked: Flutter SDK 3.47.8 not installed on the dev machine**
- [ ] Flutter app in `app/` for macOS, Windows, Linux, iOS, Android
      (`flutter create --org se.mansby --project-name passingover`)
- [ ] flutter_rust_bridge wired up: `flutter_rust_bridge_codegen integrate
      --integration-backend native-assets` (D7). Bridge crate in `app/rust`,
      added to the root workspace, with `rust-toolchain.toml` pinned to the
      same channel as the root
- [ ] "Hello from Rust core" shown in the app on macOS desktop
- [ ] `scripts/ci.sh`: Flutter stage on macOS, Linux (xvfb), Windows: `pub
      get`, `analyze`, `test`, `build <desktop> --debug`, `test
      integration_test -d <desktop>` (calls into Rust). Fail if the two
      `rust-toolchain.toml` channels differ. Linux needs clang, cmake, ninja,
      pkg-config, libgtk-3-dev and xvfb on the build host (ask first)

## M1 — Core + CLI

Data-source research is done. The core fetches the aircraft near a point,
filters them, and enriches them with route and airport data.
`passingover now --lat --lon` prints the flights overhead as short "dreamy"
lines.

Sources follow D11–D17 and docs/data-sources.md; design follows D21 (sans-IO
sources, `QueryPoint` privacy) and D22 (SI units).

Done (D21–D26):

- [x] `Coordinate`, privacy-rounded `QueryPoint`, `Radius`, distances and bearings
- [x] `Aircraft` in SI units, with an `is_flying` filter for ground traffic,
      vehicles and towers
- [x] `PositionSource` for adsb.lol and adsb.fi sharing one readsb parser
- [x] HTTP transport (`HttpClient`, `UreqClient`) and `PositionFeed`: one
      request per source per poll, backoff, rate limits, failover, random
      primary (D15, D17, D23)
- [x] Standing data: `cargo xtask data` builds routes, airports (with time
      zones) and airlines from VRS (D24); loaded and binary-searched in core
- [x] Routes: VRS callsign normalisation, plausibility check that picks the
      leg being flown, adsbdb fallback with in-memory cache and rate limit
- [x] Time-to-landing estimate (D12, D26) and local time at the destination (D25)
- [x] `passingover now --lat --lon [--radius-km] [--source] [--data]`
- [x] Recorded fixtures for positions, adsbdb and the data tables; end-to-end
      fixture test; `#[ignore]` live tests for all three services

Moved on: daily request budgets and pausing while hidden belong to the app
(M2); imperial units arrive with the first values that have units (weather,
M3); introducing the project to the source operators happens before release
(M7).

## M2 — First app view

A Flutter "Now" screen showing the flights overhead with destination, local
time there and time to landing. Runs on desktop, iOS and Android. Device
location in the app, with manual lat/lon as a fallback.

- How the app gets the standing data (bundled asset or download, D24)
- Count requests per source per day on the device and slow down near the
  limits; pause polling while the view is hidden (D15)
- Store the randomly chosen primary source once per install (D17)

## M3 — Daydream enrichment

Destination weather, the journey as a story (where it left from, how long
ago), and what it will cross.

## M4 — Ambient mode

A full-screen calm view for a desktop, spare tablet or TV, with night-sky and
LED-matrix themes and an auto-brightness schedule.

## M5 — Daydream log

Local history and gentle statistics ("this month, planes over your home went
to 61 cities").

## M6 — Native extras

macOS menu bar / tray icon; iOS and Android home-screen widgets (native Swift
or Kotlin where required).

## M7 — Release

Packaging, store listings, website.
