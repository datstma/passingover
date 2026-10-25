## What and why

<!-- What does this change, and why? Link the issue if there is one. -->

## Checklist

- [ ] `scripts/ci.sh local` passes (fmt, clippy, tests, cargo-deny)
- [ ] Logic lives in `passingover-core`, not in the bridge or the UI
- [ ] Tests use recorded fixtures, with no live network
- [ ] New data sources or dependencies are documented in `docs/data-sources.md`
      and/or `docs/decisions.md`
- [ ] No precise locations in code, tests, fixtures or logs
