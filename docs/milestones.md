# Milestones

Status values: **Not started** · **In progress** · **Done**.

| # | Milestone | Status |
|---|---|---|
| M0 | Foundation | Done |
| M1 | Core + CLI | Done |
| M2 | First app view | Done |
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
- [x] Flutter app in `app/` for macOS, Windows, Linux, iOS, Android
      (`se.mansby.passingover`), Flutter 3.47.8
- [x] flutter_rust_bridge wired up with the Native Assets backend (D7); bridge
      crate `passingover_bridge` in `app/rust`, in the root workspace (D27)
- [x] "Hello from Rust core" shown in the app on macOS desktop; integration
      test passes on macOS, Linux and Windows; macOS release build is universal
- [x] `scripts/ci.sh`: Flutter stage locally (pins, bindings, analyze, tests,
      integration test) and on Windows (analyze, tests, build, integration
      test) with a project-pinned SDK (D28)
- [x] Linux Flutter stage: analyze, tests, build and the integration test
      under xvfb pass; `scripts/ci.sh all` is green on macOS, Linux and Windows

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

- [x] Now screen: flight, "→ city", local time there, time to landing,
      origin; calm states for loading, a quiet sky, errors and no location;
      credits in the footer and an "About the data" sheet (D30)
- [x] Standing data bundled as gzipped assets (D29)
- [x] Polling every 15 ± 2 s, paused while hidden; daily request budget
      saved across restarts; primary source chosen once per install (D15,
      D17, D32)
- [x] Approximate device location, or coordinates typed in (D31)
- [x] Runs on macOS, Linux and Windows (CI), the iOS simulator and the
      Android emulator; the opt-in live test passes on macOS, iOS and Android
- [ ] Hand check on real devices: the location permission prompt and an
      actual sky full of flights (automated tests use a preset location)

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
