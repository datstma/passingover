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
*2026-10-25 · Superseded by D17*

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

### D17 — Live positions: spread the load across adsb.lol and adsb.fi
*2026-10-25 · Accepted (owner) · Supersedes D10*

- Both sources sit behind `PositionSource` and share one readsb v2 parser.
- Each install picks its primary source at random (50/50) on first launch and
  keeps it in local settings; the other source is its fallback. The choice is
  never sent anywhere. The weights are constants in core, so a release can
  shift load if an operator asks. The user can override the choice (an app
  setting; `--source` in the CLI, which otherwise picks at random per run).
- Failover follows D15: switch after 3 consecutive failures, on 401/403/429,
  or when rate-limit headers show less than 10 % remaining; retry the primary
  every 5 min.
- Stay well inside each source's stated limits (adsb.fi: 1 request/s per IP;
  adsb.lol: whatever its `x-ratelimit-*` headers say). Poll only while the
  view is visible; ambient mode is slower (D15).
- Both sources are credited on the "About data" screen, since either may serve
  a given user.
- Before the first public release, introduce the project to both operators
  (adsb.lol through its key form or IRC; adsb.fi through its listed contact),
  without blocking on a reply.

**Why:** adsb.lol is a one-person hobby service whose operator states there
are no guarantees of API access or support and doesn't answer production-use
requests (docs/data-sources.md, "Operator stance"), so the design must not
depend on an exception. With no paid tier (D16), adsb.fi's personal,
non-commercial terms fit. Splitting installs between the two roughly halves
the load on each, and either source alone can still carry everyone if the
other disappears.

### D18 — No hosted CI: checks run on the owner's machines
*2026-10-25 · Accepted (owner) · Replaces the GitHub Actions workflow from M0*

`scripts/ci.sh` runs the checks on this Mac (fmt, clippy `-D warnings`, tests,
cargo-deny), then syncs the working tree over SSH to a Linux and a Windows
build host and runs clippy and the tests there. Host names live in the
git-ignored `.ci-hosts.local`, not in the repo. The build hosts are shared
with other projects, so the script keeps everything in one directory per host
(`~/passingover-ci`: `src/`, `target/`, `inbox/`), sets env vars per command
only, installs the pinned toolchain alongside the host's others without
changing defaults, and `scripts/ci.sh clean` removes it all. On Windows it
puts the pinned toolchain's own `bin` first on PATH for the session, because
rustup's symlinked proxies can't be followed from an elevated SSH shell.

**Why:** the owner's choice; real hardware for all three desktop platforms is
at hand, and the Mac does the Apple builds. Trade-off: pull requests are not
checked automatically, so maintainers run the script before merging. GitHub
Actions is disabled in the repository settings.

### D19 — Dependency policy with cargo-deny
*2026-10-25 · Accepted*

`deny.toml` allows only licenses compatible with MIT OR Apache-2.0 (MIT,
Apache-2.0, BSD-2/3-Clause, ISC, Unicode-3.0, Zlib, CC0-1.0), fails on RustSec
advisories (vulnerable, unmaintained, yanked), denies wildcard versions and
non-crates.io sources, and warns on duplicate versions. It checks the
dependency graph for every target platform. GitHub's Dependabot security
alerts are on; Dependabot version-update PRs are not used, because no hosted
CI would check them (D18).

**Why:** keeps the dependency tree license-clean and free of known advisories,
checked on every `scripts/ci.sh` run.

### D20 — Project tools live in `.tools/`, not `~/.cargo/bin`
*2026-10-25 · Accepted*

`scripts/install-tools.sh` installs pinned project tools (currently
`flutter_rust_bridge_codegen` 2.13.0) with `cargo install --root .tools`
(git-ignored). Docs and scripts call `.tools/bin/<tool>`.

**Why:** the development machines host other projects that may need other
versions of the same tools; a repo-local install can't collide with them.

### D21 — Sans-IO sources, and privacy enforced by types
*2026-10-25 · Accepted*

A source never does I/O. `PositionSource` (and later `RouteSource`,
`WeatherSource`) builds a `Request` (a URL) for an `Area` and parses a
response body into domain types. One transport layer, still to be written,
sends requests and owns the User-Agent, timeouts, rate-limit headers, backoff
and failover (D15, D17). An `Area` can only be built from a `QueryPoint`, and
the only way to get a `QueryPoint` is to round a `Coordinate` to 2 decimals,
so a precise location can't reach a source by accident.

**Why:** fixture tests need no network or mocks; the core stays independent
of any async runtime or HTTP client, so the CLI, the Flutter bridge and a
future WASM build can each use what suits them; politeness lives in one place.

### D22 — SI units inside the core
*2026-10-25 · Accepted*

The core stores metres, metres per second, degrees and `SystemTime` (UTC).
Sources convert on the way in (readsb sends feet, knots and feet per minute),
and front ends convert on the way out: metric by default, imperial on request.
Field names carry the unit (`baro_altitude_m`, `ground_speed_mps`).

**Why:** one unit system inside rules out mixed-unit bugs; the unit in the
name makes every value self-explaining.

### D23 — Blocking HTTP with ureq, behind a cargo feature
*2026-10-26 · Accepted*

The core's `HttpClient` trait has one implementation, `UreqClient` (ureq 3,
blocking), behind the default `http` feature. It uses rustls with ring and
the bundled Mozilla root certificates (webpki-roots), gzip, an identifying
User-Agent and 5 s connect / 10 s total timeouts, and it reports every HTTP
status instead of treating 4xx/5xx as errors. Callers run it off the UI
thread; a future WASM build turns the feature off and supplies its own
client.

**Why:** a blocking client keeps the core free of an async runtime. Bundled
roots work the same on all five platforms with no per-platform certificate
setup (Android's platform verifier needs extra Kotlin/JNI wiring).

### D24 — Standing data built locally from VRS; not committed
*2026-10-26 · Accepted · Refines D14*

`cargo xtask data` shallow-clones the VRS standing data into
`target/xtask/`, adds an IANA time zone to every airport with tzf-rs, and
writes `routes.tsv` (sorted for binary search), `airports.tsv`,
`airlines.tsv`, `manifest.json` and `NOTICE.md` to the git-ignored `data/`.
The VRS airports alone cover every airport the routes use, with city
names, so OurAirports is left out until real gaps show up (D14 named it
as a gap filler). A small subset of the tables is committed as test
fixtures. How app and CLI users get the data (bundled assets, release
downloads) is decided in M2.

**Why:** the tables are ~15 MB and change daily upstream; committing them
would bloat history. The build takes seconds and records the VRS commit it
used.

### D25 — Local time with jiff and a bundled time-zone database
*2026-10-26 · Accepted*

"It's 15:12 there" uses jiff with `tzdb-bundle-always`, so the IANA
database ships inside the core.

**Why:** identical results on every platform, independent of each OS's
zoneinfo (Windows has none; Android and iOS differ).

### D26 — Landing estimate details
*2026-10-26 · Accepted · Details D12*

Within 100 km of the destination: the aircraft's ground speed (at least
120 kt) plus 5 min for landing. Further out: its ground speed if it is
cruising (FL250 or above and at least 250 kt), otherwise 450 kt, plus 15 min
for descent and approach. Rounded to 5 min and shown as "about".

**Why:** simple, explainable and good to tens of minutes on long flights,
which is all a daydream needs.

### D27 — The bridge crate: `passingover_bridge`
*2026-10-26 · Accepted*

`app/rust` is the `passingover_bridge` crate (cdylib + staticlib), a member
of the root workspace, depending on `passingover-core`. Its name uses an
underscore because it is also the native library file the Dart side loads.
It stays on edition 2021 and outside the workspace lints, because it holds
code generated by flutter_rust_bridge (D6). Its `rust-toolchain.toml` pins
the same channel as the root and lists the 12 shipping targets (no wasm).

**Why:** one Cargo.lock and one `cargo clippy/test --workspace` for all Rust
code, while the generated FFI glue keeps the settings it was generated for.

### D28 — CI hosts get their own pinned Flutter SDK
*2026-10-26 · Accepted*

`scripts/ci.sh setup linux|windows` installs Flutter 3.47.8 from the official
archive (checksum-verified) into `~/passingover-ci/flutter`, with its own
`pub-cache/`; a host's existing Flutter installs (e.g. another project's
fvm) are never used. In the synced CI copy, `app/rust/rust-toolchain.toml`
lists only the host's own target, so rustup doesn't fetch the other eleven
(about 1.4 GB) onto shared machines.

**Why:** the hosts are shared with other projects (D18); a project-owned SDK
can't collide with theirs, and `scripts/ci.sh purge` removes it cleanly.

### D29 — The app bundles the standing data
*2026-10-26 · Accepted · Settles the open point in D24*

`cargo xtask data` also writes gzipped tables (about 4 MB), the manifest and
the NOTICE to `app/assets/data/` (git-ignored), which the app declares as
assets. At start the app hands the bytes to Rust, which decompresses and
indexes them. Without them (a build without `cargo xtask data`) the app
still works and looks routes up online only. For now the data is as fresh
as the app release; downloadable updates are reconsidered before release
(M7).

**Why:** offline, no per-user downloads from anyone's server, and 4 MB is a
small price for route lookups that never leave the device.

### D30 — `Sky` in the core; the bridge hands the app ready-to-show text
*2026-10-26 · Accepted*

`passingover_core::Sky` does a whole poll (fetch, keep the flying aircraft
nearby, resolve routes, estimate landing) for every front end; the CLI and
the app share it. The bridge (`SkyHandle`) returns plain values: flight
title, destination, origin, local time, "about 7 h 40 min", the whole line,
and the credits to show. The app only lays them out. Text is English for
now; localisation comes later.

**Why:** one implementation of the logic and the wording, tested in Rust;
the Flutter side stays thin and easy to restyle.

### D31 — Approximate location only, with a manual fallback
*2026-10-26 · Accepted*

The app asks for location at low accuracy through geolocator: iOS offers
approximate location by default (`NSLocationDefaultAccuracyReduced`),
Android declares only `ACCESS_COARSE_LOCATION`, and macOS gets the location
entitlement. Users can type coordinates instead. The device location is
refreshed every 10 minutes while the app is visible, and is rounded to
~1 km before any request (D15, D21).

**Why:** the app only needs to know roughly where you are, so it shouldn't
ask for more.

### D32 — Daily request budget, and app state kept on the device
*2026-10-26 · Accepted · Details D15 and D17*

The core counts position requests per source per UTC day. A device sends at
most 450 a day to adsb.lol (90 % of the 500 its rate-limit header
announces); after that the fallback serves until midnight UTC. The app
saves the count, its randomly chosen primary source and its location
settings with shared_preferences, on the device only.

**Why:** restarts must not reset the budget, and nothing about the user
needs a server.

### D33 — "Will cross": Natural Earth countries and seas, bundled
*2026-10-26 · Accepted*

`cargo xtask data` builds `regions.tsv` from Natural Earth (public domain):
1:110m countries and 1:50m seas, sea outlines simplified to ~2 km, names
ready for a sentence ("Norway", "the North Sea", "the United States"). The
app bundles it gzipped (~180 KB). The core walks the great circle from the
aircraft to its destination in 25 km steps and names what lies ahead: it
leaves out the stretch the plane is over now and the destination's own
stretch; the countries of the origin and destination airports (matched by
ISO code, because at this scale a coastal airport such as JFK plots over the
sea) and of where the plane is wherever they reappear; seas only at the
ends, because a coastal airport can plot over the very ocean that is most
of the way. Then it merges repeats, drops stretches under 50 km and keeps
the three longest, in route order.

**Why:** "will cross the Norwegian Sea, Greenland and Canada" is the
daydream; public-domain shapes keep it offline and license-free.

### D34 — Weather: batched, cached per airport, metric by default
*2026-10-26 · Accepted · Details D13*

Each poll asks Open-Meteo once for every destination not in the cache (up
to 20 per request); if that fails, MET Norway is asked for at most three
places, one request each. Answers are cached per airport for 15 minutes
(Open-Meteo) or 30 (MET Norway, matching its `Expires`); a failing source
pauses for 5 minutes. WMO codes and MET symbols map to one set of phrases
("overcast", "raining", "snow showers"). Temperatures are °C unless the
user picks °F. Flights landing near the user show neither the time nor the
weather there: the user can look out of the window.

**Why:** a handful of requests per poll at most, nothing about the user in
them, and the sentence from the brief: "It's 15:12 there, 9°C and overcast."

### D35 — The journey: departure estimate and "landing nearby"
*2026-10-26 · Accepted*

Time since departure = distance flown from the origin at an average
420 kt, plus 10 minutes for the climb, rounded to 5 minutes; under 30 km
from the origin it "just left". A destination within 60 km of the user is
"nearby": "AA 43 → New York, landing nearby in about 5 min." The journey
sentence reads "Left Stockholm about 1 h 10 min ago and will cross …".

**Why:** like the landing estimate (D12), simple and good enough to
daydream with; no free source has actual departure times.

### D36 — Ambient mode
*2026-10-26 · Accepted*

A full-screen view for a desktop, a spare tablet or a TV, opened from the
Now screen. It shows one flight at a time, cross-fading to the next every
12 seconds, and polls every 60 seconds (D15). It keeps the screen awake
(wakelock_plus) and fills it: window_manager full screen on desktops (the
window is brought to the front first, because macOS ignores full screen for
a background window), immersive mode on phones and tablets. A dimming
schedule fades the view to 30 % at night (23:00–07:00 by default, 30-minute
fades, adjustable or off); it dims inside the app, so no system brightness
permission is needed. Two themes: "Night sky" (the Now screen's sentences in
large light type over a slowly twinkling star field) and "LED" (D37). Tap
for controls; Esc or back to leave.

**Why:** the brief's "free, software FlightWall": calm, readable across a
room, and polite to the sources even when left on all day.

### D37 — LED theme with an own 5×7 dot font
*2026-10-26 · Accepted*

The LED theme draws amber dots on a fixed grid from a small 5×7 font written
for the project (capitals, digits, the punctuation flight lines need, a
`→`). Accented letters fold to their base letter (Malmö → MALMO); other
scripts become spaces. Rows are terse ("LANDS IN 7 H 40 MIN"); lines that
don't fit scroll one whole column at a time, like a real board.

**Why:** an own font keeps the look exact on every platform with no font
licence to track; folding keeps every city readable.

### D38 — The daydream log stays on the device
*2026-10-26 · Accepted*

The core's `DaydreamLog` records each flight with a known destination once
per passage (the same aircraft and flight within two hours is one passage)
in an append-only TSV file in the app's private folder. It stores only
flight facts: when, which flight, from and to where (with the destination's
country and position). It never stores the user's location; "the farthest"
is measured from the current location at the moment the summary is shown.
The log is on by default, can be turned off and cleared, and a torn last
line (power cut) is skipped. Summaries: "This month, 1,284 planes passed
over on their way to 61 cities in 24 countries." "The farthest was going to
Singapore, about 10,000 km away." "Most often: London (34) and Paris (21)."
The app uses calendar periods (this week from Monday, this month, so far);
the CLI uses rolling ones (`now --log FILE`, `log --log FILE`).

**Why:** the "daydream log" from the brief without making the app a
location diary: what passed over, not where you were.

### D39 — macOS menu bar item, native and on by default
*2026-10-26 · Accepted*

On macOS, Passing Over shows an airplane and the nearest flight's
destination in the menu bar ("✈ Chicago"); its menu lists the flights'
sentences, when it was updated and from which source, plus Show, Ambient
mode and Quit. It is native Swift (`NSStatusItem` in
`MainFlutterWindow.swift`) driven over a method channel, with no plugin
and no new Xcode files. While the item is shown, closing the window hides
it (the Dock icon brings it back) and polling continues at the ambient
pace, once a minute: a visible menu bar item counts as "visible" for D15.
It can be turned off in settings. Windows and Linux get no tray icon for
now (the brief asks for the macOS menu bar; a Linux tray needs extra system
libraries).

**Why:** a glance at the menu bar is the calmest possible view; doing it
natively keeps the dependency list short.

### D40 — Home-screen widget shows what the app saw last
*2026-10-26 · Accepted (owner) · Android first*

The Android widget (`PassingOverWidget`, a `home_widget` provider with a
night-sky RemoteViews layout) shows the nearest flight the app saw last:
"SK 945", "→ Chicago", "15:12 there, 9°C and overcast · landing in about
7 h 40 min", "As of 21:30". The app hands it new content after polls, only
when something changed; tapping the widget opens the app. There is no
background refresh: no requests to the sources while the app is closed, and
no battery cost (`updatePeriodMillis` is 0). The iOS widget comes later; it
needs a WidgetKit extension target and an App Group, and `home_widget`
already covers the iOS side of the data hand-off.

**Why:** the owner chose "last seen by the app" over background polling:
it keeps the community sources' load where it is and the widget simple.

### D41 — Android location without Google Play services
*2026-10-26 · Accepted · Refines D31*

On Android the app asks for its approximate location through the
platform's `LocationManager` (geolocator `forceLocationManager: true`), not
Google Play services' fused provider. The fused path showed Google's
"Location Accuracy" dialog, which asks the user to share Wi-Fi scans with
Google; an app that only needs a rough position shouldn't ask for that. With
coarse permission only, Android serves network-based locations, which most
phones provide; where none is available (e.g. Google's location accuracy
turned off), the app says it couldn't find the location and offers manual
coordinates. Not depending on Play services also keeps the door open for
F-Droid (geolocator's docs show how to exclude `com.google.android.gms`).

**Why:** the privacy promise is "roughly where you are, nothing more",
including towards Google.
