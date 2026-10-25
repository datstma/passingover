# Passing Over — Flutter app

The Flutter front end for macOS, Windows, Linux, iOS and Android. All logic
lives in the Rust core (`../crates/passingover-core`); `rust/` is the thin
flutter_rust_bridge crate, built by `hook/build.dart` (D7).

```sh
flutter run -d macos
flutter test                              # widget tests
flutter test integration_test -d macos    # runs the app and calls into Rust
../.tools/bin/flutter_rust_bridge_codegen generate   # after changing rust/src/api/
```

See the repository's README, CLAUDE.md and docs/ for more.
