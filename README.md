# Passing Over

*Where the planes above you are going.*

Passing Over shows the aircraft passing overhead right now, built for
daydreaming rather than plane-spotting. Instead of callsigns and altitudes, it
tells you where each plane is going and what it's like there:

> SK 945 → Chicago. It's 15:12 there, 9°C and overcast. Landing in 7h 40m.

Planned:

- **Now**: the flights above you, with destination, local time there and
  time to landing.
- **Journey stories**: where a flight left from, how long ago, what it will cross.
- **Ambient mode**: a calm full-screen view for a desktop, a spare tablet or
  a TV.
- **Daydream log**: gentle statistics about where the planes over your home went.

Passing Over runs on macOS, Windows, Linux, iOS and Android. The UI is
Flutter; all the logic lives in a Rust core that also powers a small CLI.

## Status

Early development. The CLI works (M1), and the Flutter app runs on macOS,
Linux and Windows with the Rust core wired in (M0). The first real app screen
is M2. See
[docs/milestones.md](docs/milestones.md).

## Building

Prerequisites:

- Rust via [rustup](https://rustup.rs). The toolchain is pinned in
  `rust-toolchain.toml`; rustup installs it on first use.
- [Flutter](https://docs.flutter.dev/get-started/install) 3.47 (stable) for the app.
- Platform toolchains for the targets you build: Xcode (macOS/iOS), Visual
  Studio with "Desktop development with C++" (Windows), clang, CMake, Ninja
  and GTK 3 headers (Linux), Android SDK + JDK (Android).

CLI:

```sh
cargo xtask data       # once: build the route and airport tables in data/
cargo run -p passingover-cli -- now --lat 55.68 --lon 12.57
```

```text
SK 588 → Copenhagen. It's 00:00 there. Landing in about 10 min.
CZ 674 → Beijing. It's 07:00 there. Landing in about 7 h 30 min.

Aircraft data: adsb.lol (ODbL), https://adsb.lol. Routes: VRS standing data.
```

Your location is rounded to about 1 km before it's sent to the aircraft
data source. Route lookups that aren't in the local tables send only the
flight's callsign, never your location.

App (from `app/`):

```sh
flutter run -d macos   # or windows, linux, an iOS simulator, an Android device
```

More detail is in [CONTRIBUTING.md](CONTRIBUTING.md).

## Layout

| Path | What |
|---|---|
| `crates/passingover-core` | Rust core: data sources, filtering, enrichment, stories |
| `crates/passingover-cli` | `passingover` command-line tool |
| `app/` | Flutter app; `app/rust` is the thin flutter_rust_bridge crate |
| `design/` | Design system (to come) |
| `docs/` | Decisions, milestones, data sources |

## Data sources

Passing Over uses community and open data sources for aircraft positions,
routes, airports and weather. Their terms, limits and required attribution are
documented in [docs/data-sources.md](docs/data-sources.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
