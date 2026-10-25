# Test fixtures

Recorded API responses used by the tests. Tests never touch the network.

## positions/

Each file is one unmodified response, recorded on 2026-10-25 around
Copenhagen Airport (EKCH), a public location, with a 10 nm radius and the
User-Agent `PassingOver/0.1 (+https://github.com/datstma/passingover)`.

| File | Request |
|---|---|
| `adsb_lol_point_ekch_10nm.json` | `GET https://api.adsb.lol/v2/point/55.62/12.66/10` |
| `adsb_fi_v3_ekch_10nm.json` | `GET https://opendata.adsb.fi/api/v3/lat/55.62/lon/12.66/dist/10` |

They contain aircraft in the air, aircraft on the ground, ground vehicles and
towers, which are the cases the parser and filters need to handle.

## Licenses and attribution

These data files are **not** covered by the project's MIT/Apache-2.0 license.

- `adsb_lol_*`: data © adsb.lol contributors, made available under the
  [Open Database License 1.0](https://opendatacommons.org/licenses/odbl/1-0/)
  by [adsb.lol](https://adsb.lol).
- `adsb_fi_*`: data from [adsb.fi](https://adsb.fi), used under its open data
  terms (personal, non-commercial use, attribution with a link).

To record a new fixture, make one request with the project User-Agent at a
public location, save the body unmodified, and add it to the tables above.

## routes/

| File | Request |
|---|---|
| `adsbdb_callsign_SAS945.json` | `GET https://api.adsbdb.com/v0/callsign/SAS945` (HTTP 200) |
| `adsbdb_callsign_unknown.json` | `GET https://api.adsbdb.com/v0/callsign/ZZZ9999` (HTTP 404) |

Recorded on 2026-10-25. adsbdb's route data may not be copied into other
databases, so only these two single responses are kept, to test the format.
The route itself (SAS945, Stockholm Arlanda → Chicago O'Hare) is also in the
CC0 VRS standing data.

## data/

A few rows of each table built by `cargo xtask data` (VRS commit
`bf60597`, 2026-10-25): the routes in the position fixtures plus SAS945, and
their airports and airlines. Routes and airlines are CC0 (VRS standing data);
`airports.tsv` includes time zones derived from timezone-boundary-builder
(© OpenStreetMap contributors) and is under the
[ODbL 1.0](https://opendatacommons.org/licenses/odbl/1-0/).
