# Bundled standing data

`cargo xtask data` (run from the repository root) writes the files the app
bundles here: gzipped route, airport and airline tables, `manifest.json`
and `NOTICE.md`. They are git-ignored; this README keeps the folder in place
so the app builds without them, in which case routes are looked up online
only (D29).

Licenses: the routes and airlines are CC0 (VRS standing data); `airports.tsv`
includes time zones derived from timezone-boundary-builder
(© OpenStreetMap contributors) and is under the ODbL 1.0. See `NOTICE.md`.
