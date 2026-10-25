# Milestones

Status values: **Not started** · **In progress** · **Done**.

| # | Milestone | Status |
|---|---|---|
| M0 | Foundation | In progress |
| M1 | Core + CLI | Not started |
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
Linux and Windows.

- [x] Git repository on `main` (no remote)
- [x] Licenses (MIT, Apache-2.0)
- [x] Rust workspace: `passingover-core`, `passingover-cli` (`passingover hello`)
- [x] Docs: CLAUDE.md, README, CONTRIBUTING, decisions, milestones, design placeholder
- [ ] docs/data-sources.md: first research pass
- [ ] Flutter app in `app/` for macOS, Windows, Linux, iOS, Android
- [ ] flutter_rust_bridge wired up (`app/rust`, Native Assets backend)
- [ ] "Hello from Rust core" shown in the app on macOS desktop
- [ ] CI: Rust (fmt, clippy, test) and Flutter (analyze, test, desktop build,
      Rust-calling integration test) on macOS, Linux, Windows

## M1 — Core + CLI

Data-source research is done. The core fetches the aircraft near a point,
filters them, and enriches them with route and airport data.
`passingover now --lat --lon` prints the flights overhead as short "dreamy"
lines.

- `PositionSource`, `RouteSource` (and later `WeatherSource`) traits; one
  implementation each, chosen in docs/decisions.md
- Politeness: ~10–15 s polling, small default radius, caching, backoff
- Privacy: coordinates rounded to ~1 km before leaving the device
- Bundled or cached airport data with IANA time zones
- Unit tests on recorded JSON fixtures; one opt-in live integration test

## M2 — First app view

A Flutter "Now" screen showing the flights overhead with destination, local
time there and time to landing. Runs on desktop, iOS and Android. Device
location in the app, with manual lat/lon as a fallback.

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
