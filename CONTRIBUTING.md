# Contributing to Passing Over

Thanks for your interest! Passing Over is early. The best way to help right
now is to open an issue before starting anything large, so we can agree on
the direction.

## Set up

1. Install Rust with [rustup](https://rustup.rs). The repo pins its toolchain
   in `rust-toolchain.toml`, and rustup fetches it on first use, along with
   the cross-compilation targets listed in `app/rust/rust-toolchain.toml`.
2. Install [Flutter](https://docs.flutter.dev/get-started/install) 3.47
   (stable) and run `flutter doctor` for your platform.
3. If you change the bridge API, install the code generator at the same
   version as the `flutter_rust_bridge` dependency:

   ```sh
   cargo install flutter_rust_bridge_codegen --version 2.13.0 --locked
   ```

## Everyday commands

```sh
# Rust (repo root)
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p passingover-cli -- hello

# Flutter (app/)
flutter pub get
flutter analyze
flutter test
flutter test integration_test -d macos   # or -d linux / -d windows
flutter run -d macos

# Regenerate bindings after editing app/rust/src/api/
flutter_rust_bridge_codegen generate
```

CI runs the same checks on macOS, Linux and Windows.

## Where code goes

- **Logic belongs in `crates/passingover-core`**, which has no Flutter
  dependency. The bridge crate in `app/rust` only adapts types and forwards
  calls.
- Data sources sit behind traits so they can be swapped. Adding one requires
  documenting its terms, rate limits and attribution in
  [docs/data-sources.md](docs/data-sources.md).
- Tests must not touch the network. Use recorded JSON fixtures. A live test
  is allowed only if it's `#[ignore]`d (run it with `cargo test -- --ignored`).
- Respect user privacy: round coordinates to ~1 km before sending them
  anywhere, and never log precise locations.

## Commits and pull requests

- Keep commits small and focused, with a clear message (an imperative summary
  line, then the why).
- For any non-trivial choice (a dependency, a data source, an architectural
  change), add an entry to [docs/decisions.md](docs/decisions.md).
- Make sure `cargo fmt`, `cargo clippy`, `cargo test`, `flutter analyze` and
  `flutter test` pass before opening a PR.

## License

By contributing, you agree that your contributions will be dual-licensed under
the MIT and Apache-2.0 licenses, as described in the [README](README.md#license),
without any additional terms or conditions.
