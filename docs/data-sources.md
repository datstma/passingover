# Data sources

Researched: 2026-10-25. First research pass for M1. Every claim below was
checked against the provider's own page, API docs, source code or GitHub README
on that date, or against a small live request. Anything that could not be
checked that way is marked **unverified**. Terms change, so re-check a source
before relying on it.

## What we need

For each aircraft overhead, Passing Over needs:

1. **Live position** (ADS-B): hex/ICAO24, callsign, position, altitude, speed,
   track, vertical rate, on-ground, registration, type, timestamp.
2. **Route** (origin and destination from the callsign) and ideally a
   scheduled or estimated arrival time.
3. **Weather at the destination.**
4. **Airport data**: ICAO/IATA, name, city, country, coordinates, IANA time
   zone. We want to bundle this offline.

Constraints that shape the choice:

- **Requests go straight from each user's device to the source.** There is no
  proxy in v1. Per-IP limits apply per user, but there could be thousands of
  installs, and many mobile users share one IP behind carrier NAT.
- **No API keys in v1.** A key cannot be kept secret inside a distributed app,
  so any source that needs one needs a proxy first.
- **Polite by default:** poll every ~10–15 s, small radius, cache, back off.
- **Privacy:** user coordinates rounded to ~1 km before they leave the device.
- **Distribution:** app stores; a paid tier may come later. Non-commercial-only
  terms are a problem for that.
- **Licensing:** the app is MIT OR Apache-2.0 (D2). Bundled data under ODbL or
  CC BY-SA brings obligations of its own (see each section).
- Planned features that affect volume: the ambient mode (M4) runs all day on a
  spare device, and the daydream log (M5) keeps local history.

## Summary

| Source | Category | Auth / key | Rate limit | License / terms | Commercial use | Attribution required | Direct from client OK? | Verdict |
|---|---|---|---|---|---|---|---|---|
| [adsb.lol](#adsblol) | Positions (+ routes, airports) | None today; docs say a key "by feeding" will be required in future | Headers seen: 4/s, 60/min, **500/day** per client (enforcement of the daily figure unverified) | ODbL 1.0 | Allowed by ODbL; operator asks production users to contact them | Yes (ODbL) | Technically yes; no statement on apps; "production" use: contact | **Primary for positions**, contact the operator |
| [adsb.fi](#adsbfi) | Positions | None | 1 req/s (public endpoints); 4xx responses count | adsb.fi terms | **No**: personal, non-commercial only | Yes: cite adsb.fi and link to it | Not addressed; personal use OK | **Fallback for positions** while free/non-commercial |
| [OpenSky Network](#opensky-network) | Positions | Anonymous allowed for `/states/all`; accounts use OAuth2 client credentials | Anonymous: 400 credits/day per IP (≈400 small requests) | OpenSky terms | **No** without written licence | Cite paper in publications | Terms appear to require a written agreement for use in a live product (**unverified**, page blocked) | Not for v1 |
| [airplanes.live](#airplaneslive) | Positions | Our requests got HTTP 403 "contact us" | n/a | Non-commercial (**unverified**) | Unverified | Unverified | **No** (refused in test) | Not usable |
| [ADS-B Exchange](#ads-b-exchange) | Positions | RapidAPI key, US$10/month | 10,000 requests | Non-commercial on that plan | Enterprise licence | — | No (key) | Not usable in v1 |
| [VRS standing data](#vrs-standing-data) | Routes, airports, airlines | None (bundle) | n/a | **CC0** | Yes | No | Bundle; no per-user requests | **Primary for routes and airport base** |
| [adsbdb](#adsbdb) | Routes, aircraft | None | 512 requests / rolling 60 s per IP | Code MIT; route data may not be copied or put in other databases without permission | Unclear | Credit route data authors | Online lookups OK; do not bundle or keep long-term | **Fallback for routes** (online, short cache) |
| [adsb.lol routes](#adsblol-routes) | Routes | None | As adsb.lol | Data is VRS (CC0) | — | — | `routeset` returned empty in tests | Not needed if we bundle VRS |
| [hexdb.io](#hexdbio) | Routes | None | Not stated; IPs blocked on excess | No license stated | Unclear | — | — | Not recommended |
| [Open-Meteo](#open-meteo) | Weather (+ tz) | None (free); key for paid | 600/min, 5,000/h, 10,000/day, 300,000/month | Data CC BY 4.0; free API **non-commercial only** | Paid plan + API key | Yes: link next to the data | Yes for free non-commercial use | **Primary for weather (v1)** |
| [MET Norway](#met-norway-locationforecast) | Weather | None; identifying User-Agent **required** | 20 req/s **total** across all installs; honour `Expires` | NLOD 2.0 + CC BY 4.0 | Yes | Yes: "Data from MET Norway" | Discouraged; low-volume apps allowed if identified | **Fallback for weather**; commercial-safe |
| [aviationweather.gov METAR](#aviationweathergov-metar) | Observed weather at airports | None | 100 req/min | US government service; no license stated on the API page (**unverified**) | Probably yes (**unverified**) | Not stated | Yes (set a User-Agent) | Optional complement |
| [OurAirports](#ourairports) | Airports | None (bundle) | n/a | **Public domain** | Yes | No (credit welcome) | Bundle | Airport enrichment; **no tz column** |
| [mwgg/Airports](#mwggairports) | Airports with tz | None (bundle) | n/a | MIT | Yes | MIT notice | Bundle | Tz cross-check / fallback |
| [OpenFlights](#openflights) | Airports, routes | None | n/a | ODbL + DbCL | Paid licence on request | Yes | Bundle | Not recommended (stale) |
| [tzf-rs](#tzf-rs-time-zone-from-coordinates) | Tz from coordinates | None (crate) | n/a | Code MIT; data **ODbL** | Yes, with ODbL duties | Yes (ODbL) | Offline | **Primary for tz lookup** |

---

## Live aircraft positions

The hobbyist aggregators (adsb.lol, adsb.fi, airplanes.live) all serve the
readsb / ADS-B Exchange v2 JSON shape, so one parser covers them. OpenSky uses
its own array format.

### Field mapping

Units for the v2 shape are from the readsb JSON docs
(<https://github.com/wiedehopf/readsb/blob/dev/README-json.md>). OpenSky units
are from its REST docs.

| We need | v2 shape (adsb.lol, adsb.fi) | OpenSky state vector |
|---|---|---|
| hex / icao24 | `hex` | `[0] icao24` |
| callsign | `flight` (8 chars, space-padded; trim) | `[1] callsign` (8 chars, may be null) |
| lat / lon | `lat`, `lon` | `[6] latitude`, `[5] longitude` |
| altitude | `alt_baro` (ft, or the string `"ground"`), `alt_geom` (ft) | `[7] baro_altitude` (m), `[13] geo_altitude` (m) |
| ground speed | `gs` (knots) | `[9] velocity` (m/s) |
| track | `track` (degrees true) | `[10] true_track` (degrees) |
| vertical rate | `baro_rate`, `geom_rate` (ft/min) | `[11] vertical_rate` (m/s) |
| on ground | `alt_baro == "ground"` | `[8] on_ground` |
| registration | `r` | not in state vector |
| aircraft type | `t` (ICAO type designator); adsb.fi also has `desc` | not in state vector (`category` only with `extended=1`) |
| timestamp | top-level `now` (ms epoch in these APIs) minus `seen` / `seen_pos` (seconds) | `[3] time_position`, `[4] last_contact` (Unix s) |
| distance / bearing from query point | `dst` (nm), `dir` (degrees) | compute ourselves |

Notes: `lat`/`lon` older than 60 s are dropped by readsb and moved to
`lastPosition`. Ground vehicles and towers appear too (`t: "TWR"`, category
`C0`–`C2`, type `adsb_icao_nt`); filter on `category` `A*`/`B*` or on altitude.

### adsb.lol

- **Endpoint (radius):** `GET https://api.adsb.lol/v2/point/{lat}/{lon}/{radius_nm}`
  (alias `/v2/lat/{lat}/lon/{lon}/dist/{radius}`). Radius is an integer,
  0–250 nm per the OpenAPI schema. Also `/v2/closest/{lat}/{lon}/{radius}`,
  `/v2/callsign/{cs}`, `/v2/hex/{hex}`.
- **Example:**
  ```sh
  curl -A 'PassingOver/0.1 (+https://github.com/datstma/passingover)' \
    https://api.adsb.lol/v2/point/55.62/12.66/25
  ```
  Live test returned `{"ac":[…], "msg":"No error", "now":1792963931008, "total":14, …}`
  with all fields in the table above, including `r` and `t`.
- **Rate limit:** the API README says limits are "dynamic based on the
  environment load". Every response we saw carried
  `x-ratelimit-limit: 4, 4;w=1, 60;w=60, 500;w=86400`, i.e. 4 per second,
  60 per minute and **500 per day**. If the daily figure is enforced per IP, a
  15 s poll uses it up in about 2 h of viewing; the ambient mode (M4) would hit
  it early in the day. Read these headers at runtime rather than hard-coding.
- **Auth:** none today. The API description says a future key will come from
  feeding adsb.lol. The site also has an ad-hoc key request form
  (<https://www.adsb.lol/request>) that asks about volume, commercial use and
  a budget for service costs.
- **License:** ODbL 1.0 for the API and all data adsb.lol publishes.
- **Terms:** the API description says use is free and adds:
  "If you want to use the API for production purposes, please contact me".
  Nothing about distributed apps or User-Agent. Privacy page: temporary logs
  kept under 30 days.
- **ODbL in practice:** showing positions in the app is a work produced from
  the database, so the app must credit adsb.lol and name the ODbL. Keeping data
  on the device (cache, the M5 log) is not public use. If we ever publish a
  database derived from adsb.lol data (for example a route table learned from
  observed flights, or a shared cache on a future proxy), that database must be
  offered under the ODbL. The app's MIT/Apache code is not affected.
- **Attribution text (proposal):** "Aircraft data: adsb.lol (ODbL)" with links
  to <https://adsb.lol> and <https://opendatacommons.org/licenses/odbl/1-0/>.
- **Risks:** daily cap; key requirement announced; one-person operation.
- **Checked:** <https://api.adsb.lol/docs> (OpenAPI at
  <https://api.adsb.lol/api/openapi.json>), <https://github.com/adsblol/api>,
  <https://www.adsb.lol/docs/open-data/api/>,
  <https://www.adsb.lol/docs/privacy-license/>, <https://www.adsb.lol/request>.

### adsb.fi

- **Endpoint (radius):** `GET https://opendata.adsb.fi/api/v3/lat/{lat}/lon/{lon}/dist/{nm}`,
  up to 250 nm. The v2 form `/v2/lat/…/lon/…/dist/…` is deprecated and has a
  different response shape; use v3.
- **Example:**
  ```sh
  curl -A 'PassingOver/0.1 (+https://github.com/datstma/passingover)' \
    https://opendata.adsb.fi/api/v3/lat/55.62/lon/12.66/dist/25
  ```
  Live test returned `{"ac":[…], "now":…, "total":15, …}`; same fields as
  adsb.lol plus `desc` (type description). No rate-limit headers.
- **Rate limit:** 1 request per second on public endpoints. Requests that
  return 400, 401, 403, 404 or 429 count toward the limit, and many invalid
  requests trigger a temporary IP restriction. No daily cap is stated.
- **Auth:** none.
- **Terms:** "adsb.fi open data is for personal, non-commercial use only." No
  licensing, selling or renting of the data; the service can be suspended at
  any time. Commercial or higher-rate use: contact them.
- **Attribution:** required: cite adsb.fi and link to its home page.
  Proposal: "Aircraft data: adsb.fi" linking to <https://adsb.fi>.
- **Direct from client:** not addressed. A free app used by individuals is
  arguably personal use, but app-store distribution and any paid tier are
  not; ask before shipping.
- **Checked:** <https://github.com/adsbfi/opendata> (README last updated
  2026-05-17).

### OpenSky Network

- **Endpoint (area):** `GET https://opensky-network.org/api/states/all?lamin=&lomin=&lamax=&lomax=`
  (bounding box, not radius). All four bounds are needed together.
- **Example (anonymous):**
  ```sh
  curl 'https://opensky-network.org/api/states/all?lamin=55.3&lomin=12.1&lamax=55.9&lomax=13.2'
  ```
  Live test returned 200 with `x-rate-limit-remaining: 399`.
- **Auth model (verified):** OAuth2 client credentials only; basic auth is no
  longer accepted. Accounts created since mid-March 2025 needed OAuth2 from the
  start; the docs were changed to "OAuth2 only" on 2026-03-05. Tokens last 30
  minutes. Anonymous access still works for `/states/all`: latest state only,
  10 s resolution, credits bucketed by IP.
- **Credits:** anonymous 400/day, standard user 4,000/day, active feeder
  8,000/day, licensed 14,400/hour. `/states/all` costs 1 credit for a box of
  ≤25 square degrees (our 25 nm box at 55°N is about 1.2 sq°). So anonymous
  use allows about 400 polls per day per IP: under 2 h at 15 s. `429` plus
  `X-Rate-Limit-Retry-After-Seconds` when exhausted.
- **Routes / flights:** `/flights/*` is batch-processed (previous day or
  earlier), so it cannot give a live destination. The old `/api/routes`
  endpoint returned 404.
- **Terms:** the API docs say the API is for research and non-commercial
  purposes, and: "If you want to retrieve live flight information for
  commercial purposes, please contact us". Search-engine excerpts of the
  terms page and FAQ say that any for-profit use needs a written licence and
  that operational use, including integration into a live product, needs a
  prior written agreement even for non-profits. **Unverified:** the terms page
  and FAQ returned 403 to our fetches. Registering several accounts gets all of
  them banned.
- **Attribution:** cite the 2014 OpenSky paper in publications; the docs also
  suggest naming <https://opensky-network.org>.
- **Verdict:** a keyless fallback in a pinch, but the daily credits are small
  and the terms appear to need an agreement for a shipped app.
- **Checked:** <https://openskynetwork.github.io/opensky-api/rest.html>,
  <https://openskynetwork.github.io/opensky-api/>,
  <https://github.com/openskynetwork/opensky-api> (docs history),
  <https://opensky-network.org/about/terms-of-use> (403),
  <https://opensky-network.org/about/faq> (403).

### airplanes.live

- `GET https://api.airplanes.live/v2/point/{lat}/{lon}/{radius}` returned
  **HTTP 403** on 2026-10-25 with any User-Agent, with the body
  `{"error": "Please contact us at contact@airplanes.live. …"}` asking for a
  project description and links.
- The API guide and terms pages are behind a Cloudflare browser challenge and
  could not be read. Search excerpts say the API is non-commercial with no SLA,
  and a forum report says the free API is now limited to feeder IPs. All of
  that is **unverified**.
- Their GitHub `api-archive` repo is the former ADSB One API (1 req/s), last
  touched 2023.
- **Verdict:** not usable without an arrangement with the operator.
- **Checked:** <https://airplanes.live/api-guide/> (challenge page),
  <https://github.com/airplanes-live/api-archive>.

### ADS-B Exchange

The Community API (formerly API Lite) is sold through RapidAPI at US$10/month
for 10,000 requests, needs a key, and is for non-commercial use. Commercial use
goes through their Enterprise products. Not usable in v1.
Checked: <https://www.adsbexchange.com/community/developer-hub/>.

---

## Routes (origin and destination by callsign)

### How reliable callsign routes are

- All free sources map a **callsign** to a route that someone entered by hand
  (crowd-sourced). They know nothing about today's actual flight.
- They go wrong for charters, positioning flights, diversions, seasonal
  changes, airlines that reuse a callsign for another route, and multi-leg
  routes (`A-B-C`), where we must pick the leg in progress.
- Pilots type the callsign, so it can be inconsistent; VRS normalises
  callsigns (see its README) and we should apply the same rules.
- Some data is old: hexdb.io returned `updatetime` 1680179818 (March 2023) for
  SAS945.
- **Mitigation:** a plausibility check like adsb.lol's: accept the route only
  if the aircraft is near the great circle between origin and destination and
  its track points roughly toward the destination. Otherwise show "route
  unknown" or mark it as a guess.
- Spot check on 2026-10-25 near CPH: SAS47D (LIRN-EKCH), NSZ3215 (ESSA-EKCH),
  NSZ9GY (LFPG-EKCH) and CSN346 (EHAM-ZBAD) were all consistent with what the
  aircraft were doing.

### VRS standing data

- **What:** CSV dumps of the Virtual Radar Server SDM database: routes,
  airports, airlines, aircraft, model types.
  <https://github.com/vradarserver/standing-data>
- **License:** **CC0 1.0** (the repo LICENSE file). No attribution needed;
  route contributors are listed in `routes/schema-01/CREDITS.md`.
- **Update cadence:** automated "Standing data changes" commit every day
  (latest 2026-10-25).
- **Routes:** `routes/schema-01/{first letter}/{CODE}-all.csv` (or
  `{CODE}-{digit}.csv` for codes with >10,000 routes). Columns: `Callsign`,
  `Code`, `Number`, `AirlineCode`, `AirportCodes` (hyphen-separated, in flight
  order). Example: `SAS945,SAS,945,SAS,ESSA-KORD`.
- **Airports:** `airports/schema-01/{letter}/{XX}.csv`; columns `Code`,
  `Name`, `ICAO`, `IATA`, `Location`, `CountryISO2`, `Latitude`, `Longitude`,
  `AltitudeFeet`. **No time zone.** Route airport codes are guaranteed to exist
  in this file, so the join always works.
- **Airlines:** `airlines/schema-01/airlines.csv` with `ICAO` and `IATA`, which
  lets us show `SAS945` as "SK 945".
- **Size:** routes ≈19 MB of CSV in 1,576 files; airports ≈2.5 MB; airlines
  ≈180 KB (uncompressed; compresses well).
- **How to use:** bundle a compressed snapshot in the app and refresh it
  occasionally from a release asset we publish (a CI job can build it from
  this repo). No per-user requests, so no rate or privacy issue.
- **Per-callsign mirror:** adsb.lol hosts the same data as JSON, with airport
  details included, at `https://vrs-standing-data.adsb.lol/routes/{first 2 chars}/{CALLSIGN}.json`
  (tested with SAS47D). Useful for development; for the app, bundling is
  better than thousands of clients calling a hobby host.

### adsbdb

- **Endpoints:** `GET https://api.adsbdb.com/v0/callsign/{CALLSIGN}`;
  `GET https://api.adsbdb.com/v0/aircraft/{MODE_S or REG}?callsign={CALLSIGN}`
  (aircraft plus route in one call). Unknown values return 404.
- **Example:**
  ```sh
  curl -A 'PassingOver/0.1 (+https://github.com/datstma/passingover)' \
    https://api.adsbdb.com/v0/callsign/SAS945
  ```
  Returned origin ESSA/ARN Stockholm-Arlanda, destination KORD/ORD Chicago
  O'Hare, airline SAS/SK, each airport with name, municipality, country,
  lat/lon and elevation. No times, no time zone.
- **Rate limit:** rolling 60 s window per IP; more than 512 requests blocks
  for 60 s, more than 1,024 for 300 s (site text and `ratelimit.rs` in the
  source).
- **Auth:** none.
- **License / terms:** code MIT. The route data notice says the data is the
  work of David Taylor and Jim Mason and "may not be copied, published, or
  incorporated into other databases without the explicit permission" of David
  J Taylor. Aircraft data is credited to PlaneBase. There is no statement about
  commercial use.
- **What that means for us:** live lookups are fine; do not bundle adsbdb
  routes or keep them in a long-lived local database without permission. A
  short-lived cache is probably acceptable but is not stated.
- **Checked:** <https://github.com/mrjackwills/adsbdb>,
  <https://github.com/mrjackwills/adsbdb_site> (rate limit and credits text),
  <https://api.adsbdb.com/v0/callsign/SAS945>.

### adsb.lol routes

- `POST https://api.adsb.lol/api/0/routeset` with
  `{"planes":[{"callsign":"SAS47D","lat":55.676,"lng":12.746}]}` (max 100
  planes). The source code returns route objects with a `plausible` flag, but
  three attempts on 2026-10-25 got **HTTP 201 with an empty body**.
- `GET /api/0/route/{callsign}` is deprecated (redirects to the VRS mirror
  after a 5 s delay).
- The data is VRS standing data, so bundling VRS gives the same result without
  a network call.

### hexdb.io

`GET https://hexdb.io/api/v1/route/icao/{callsign}` returns e.g.
`{"flight":"SAS945","route":"ESSA-KORD","updatetime":1680179818}`. No API key,
no stated limit (IPs blocked on excess), no license stated, route source not
named. Not recommended. Checked: <https://hexdb.io/>.

### Scheduled arrival time and flight progress

There is **no free, keyless source** of scheduled or estimated arrival times.

- OpenSky `/flights/*` covers the previous day or earlier only.
- AeroDataBox: key needed; free "Basic" plan on RapidAPI with 400 units/month,
  1 req/s (<https://aerodatabox.com/pricing/>). Which endpoint gives arrival
  times for a callsign is **unverified**.
- aviationstack: key needed; free plan is 100 requests/month, non-commercial
  (<https://aviationstack.com/pricing>).
- Both would need a proxy (keys) and their quotas are far too small for
  per-user calls.

**Fallback (recommended for v1):** estimate.

1. Remaining distance = great-circle distance from the current position to the
   destination airport.
2. Time = remaining distance ÷ ground speed, using `gs` in cruise. While
   climbing or below ~FL250 use a typical cruise speed (≈450 kt) instead, since
   `gs` is low then.
3. Add a fixed approach allowance (~15 min) for descent and the approach path.
4. Show it as approximate ("about 7 h 40 m") and round to 5 min. Expect errors
   of tens of minutes on long flights (winds, routing such as North Atlantic
   tracks).

---

## Weather at the destination

Weather requests use the **destination airport's** coordinates, never the
user's, so they add no location privacy risk. They do reveal the user's IP.

### Open-Meteo

- **Endpoint:** `GET https://api.open-meteo.com/v1/forecast?latitude=&longitude=&current=temperature_2m,weather_code,cloud_cover,is_day&timezone=auto`
- **Example:**
  ```sh
  curl 'https://api.open-meteo.com/v1/forecast?latitude=41.98&longitude=-87.90&current=temperature_2m,weather_code,cloud_cover,is_day&timezone=auto'
  ```
  Returned `"timezone":"America/Chicago"`, `"utc_offset_seconds":-18000`, and
  `current` with `time` (local), `interval` 900 (15 min), `temperature_2m`
  (°C), `weather_code` (WMO code; 3 = overcast), `cloud_cover` (%), `is_day`.
  With `timezone=auto` it also gives the destination's IANA time zone, which
  is a useful runtime cross-check.
- **Limits (free):** fewer than 10,000 calls per day, 5,000 per hour, 600 per
  minute; 300,000 per month in the pricing table. Limits apply "by IP
  addresses and application"; it is unclear whether many installs of one app
  are ever counted together.
- **Auth:** none for the free API. Paid plans use
  `https://customer-api.open-meteo.com/…&apikey=…`.
- **Terms:** "You may only use the free API services for non-commercial
  purposes." Their examples: apps with subscriptions or advertising are
  commercial; private or non-profit apps without them are not; tip jars and
  voluntary donations are allowed.
- **Commercial use:** needs a subscription (Standard 1M calls/month,
  Professional 5M, Enterprise 50M+), which also comes with a key, so a proxy.
  Price: about €/US$29 per month for Standard according to their blog and
  third-party listings; the official pricing page renders prices with
  JavaScript and we could not read them (**unverified**).
- **License / attribution:** data is CC BY 4.0. Their licence page requires a
  link next to any place the data is shown, e.g.
  `<a href="https://open-meteo.com/">Weather data by Open-Meteo.com</a>`.
  Some underlying models have their own licences (e.g. UK Met Office CC BY-SA);
  Open-Meteo lists them on its licence page.
- **Privacy:** free-API server logs can include coordinates and are deleted
  after 90 days.
- **Checked:** <https://open-meteo.com/en/terms>,
  <https://open-meteo.com/en/pricing>, <https://open-meteo.com/en/licence>,
  <https://open-meteo.com/en/docs>.

### MET Norway Locationforecast

- **Endpoint:** `GET https://api.met.no/weatherapi/locationforecast/2.0/compact?lat=&lon=`
  (max 4 decimals; 5+ decimals returns 403 on new products).
- **Example:**
  ```sh
  curl --compressed -A 'PassingOver/0.1 (+https://github.com/datstma/passingover)' \
    'https://api.met.no/weatherapi/locationforecast/2.0/compact?lat=41.9786&lon=-87.9048'
  ```
  Returned `properties.timeseries[]` with `data.instant.details`
  (`air_temperature` °C, `cloud_area_fraction` %, wind, humidity, pressure)
  and `next_1_hours.summary.symbol_code` (e.g. `partlycloudy_day`). Response
  had `Expires` 30 min after `Last-Modified`.
- **Identification (required):** every request must carry an identifying
  User-Agent with the app name and a contact email or a link to a site with
  contact details; their example is `AcmeWeatherApp/0.9 github.com/acmeweatherapp`.
  Without it you risk being blocked; fake or random strings get a permanent
  ban.
- **Limits:** more than 20 requests/second **per application, total across
  all installations**, needs a special agreement. Do not repeat requests before
  `Expires`; use `If-Modified-Since`; gzip and redirects must be supported;
  don't schedule requests on the hour; mobile apps must not fetch while not in
  use; 429 means throttled.
- **Direct from client:** "Browsers and mobile apps should not contact the API
  directly". Low-volume apps may, if they identify themselves; heavy traffic
  must go through a caching proxy. MET also notes user IPs and coordinates end
  up in their logs and recommends a proxy for anonymity.
- **License / attribution:** NLOD 2.0 and CC BY 4.0. Credit MET Norway, e.g.
  "Data from MET Norway" or "Based on data from MET Norway", with a link.
  Don't use "Yr" in the name or the Yr logo. Weather icons are MIT.
- **Commercial use:** no restriction stated; there is no SLA.
- **Checked:** <https://api.met.no/doc/TermsOfService>,
  <https://api.met.no/doc/License>,
  <https://api.met.no/weatherapi/locationforecast/2.0/documentation>.

### aviationweather.gov METAR

The destination is an airport, so its METAR (observed weather, usually
updated every 30–60 min) is a natural source for "9°C and overcast".

- **Endpoint:** `GET https://aviationweather.gov/api/data/metar?ids=KORD,EKCH&format=json`
  (several stations per call). Returned `temp`, `dewp`, `wdir`, `wspd`,
  `visib`, `cover` (e.g. `BKN`, `OVC`), `clouds[]`, `rawOb`, `obsTime`, `name`.
  `Cache-Control: max-age=60`.
- **Limits:** "All requests are rate limited to 100 requests per minute."
  Keep requests narrow; for bulk use, gzipped cache files of all METARs are
  updated every minute. CORS is not allowed (only matters for a web build).
- **Auth:** none. Setting a custom User-Agent is recommended.
- **Coverage:** worldwide METARs.
- **License:** US government (NOAA) service. The Data API page states no
  license. Its sibling, the NWS weather.gov API, describes its data as open
  and free to use for any purpose, but that page does not cover
  aviationweather.gov. Treat the license, especially for non-US METARs
  (relayed WMO data), as **unverified**.
- **Checked:** <https://aviationweather.gov/data/api/>.

Also seen: the US National Weather Service API (`api.weather.gov`) needs a
User-Agent with contact details and has unpublished rate limits; US only, so
not a general source. <https://www.weather.gov/documentation/services-web-api>

---

## Airport data

### OurAirports

- **License:** "All data is released to the Public Domain", no warranty.
  Credit is welcome but not required.
- **Updates:** nightly CSV dump (latest `airports.csv` 2026-10-25, 12.7 MB).
- **Fields (`airports.csv`):** `id`, `ident`, `type` (large/medium/small
  airport, heliport, seaplane base, balloonport, closed), `name`,
  `latitude_deg`, `longitude_deg`, `elevation_ft`, `continent`,
  `iso_country`, `iso_region`, `municipality`, `scheduled_service`,
  `icao_code`, `iata_code`, `gps_code`, `local_code`, `home_link`,
  `wikipedia_link`, `keywords`. **No time zone column** (still true).
- **Use:** filter to rows with an IATA code or `scheduled_service=yes` to keep
  the bundle small; use it to fill gaps in VRS airports (region, type).
- **Checked:** <https://ourairports.com/data/>,
  <https://ourairports.com/help/data-dictionary.html>,
  <https://github.com/davidmegginson/ourairports-data>.

### mwgg/Airports

- One JSON file (`airports.json`, ≈9 MB, ~29k entries keyed by ICAO) with
  `icao`, `iata`, `name`, `city`, `state`, `country`, `elevation`, `lat`,
  `lon`, **`tz`** (IANA, e.g. `America/Chicago`).
- **License:** MIT. Maintained through pull requests (latest 2026-10-01).
- **Caveats:** the README says time zones came from TimeZoneDB and were updated
  using TimeAPI; it does not say where the airport records themselves came
  from. Those upstream terms were not checked (**unverified**).
- **Use:** cross-check our computed time zones, or as a fallback.
- **Checked:** <https://github.com/mwgg/Airports>.

### OpenFlights

- **License:** ODbL with DbCL for contents; commercial licences on request
  (from US$100 for simple cases). Using it requires attribution and releasing
  public derived databases under a free licence.
- **Staleness:** airports "as of January 2017"; route data stopped updating in
  June 2014 and is historical only; DST data is a 2009 country-level
  approximation. GitHub copies are sporadic snapshots.
- Has a `Tz database time zone` column, but given the age and ODbL share-alike
  it adds nothing over OurAirports + tzf-rs. **Not recommended.**
- **Checked:** <https://openflights.org/data.php>.

### tzf-rs (time zone from coordinates)

- Rust crate (v2.1.3, code MIT) that maps lat/lon to an IANA time zone name.
  The default `bundled` feature embeds a ~4 MB "lite" dataset; simplified
  borders stay within ~111 m of the full-precision ones. A 14 MB full dataset
  exists but only from git.
- **Data license: ODbL.** The data (`tzf-dist`) is built from
  timezone-boundary-builder, whose output is ODbL and comes from OpenStreetMap.
- **Obligations if bundled in the app:** include an attribution and ODbL
  notice, e.g. "Time zone boundaries: timezone-boundary-builder, © OpenStreetMap
  contributors, ODbL". Shipping the unmodified dataset does not put the app's
  code under the ODbL.
- **Build-time alternative:** compute `tz` for each bundled airport in CI and
  ship only the airport table. Whether that table is a "produced work" (needs
  attribution only) or an adapted database (needs ODbL share-alike) is not
  clear-cut. The safe route is to publish the generated airport table under the
  ODbL as a separate data file with attribution. That costs us nothing because
  it is open anyway, and the app code stays MIT/Apache.
- **Converting to local time:** `chrono-tz` (MIT OR Apache-2.0) turns the IANA
  name into an offset for the current instant.
- **Checked:** <https://github.com/ringsaturn/tzf-rs>,
  <https://github.com/ringsaturn/tzf-dist> (`LICENSE_DATA` is the ODbL),
  <https://github.com/evansiroky/timezone-boundary-builder> (Licenses section),
  <https://opendatacommons.org/licenses/odbl/summary/>.

---

## Recommendation for v1

| Category | Primary | Fallback | Why |
|---|---|---|---|
| Positions | **adsb.lol** `/v2/point` | **adsb.fi** `/api/v3/lat/…/lon/…/dist/…` | Same JSON shape, one parser. adsb.lol's ODbL allows commercial use and has the clearest license; adsb.fi has no daily cap but is personal/non-commercial only. Email the adsb.lol operator before release (production use, the 500/day header, the planned key). Drop or renegotiate adsb.fi if a paid tier comes. |
| Routes | **VRS standing data**, bundled (CC0) | **adsbdb** online lookup, short in-memory cache | VRS is public domain, updated daily, and offline: no per-user requests and no location leaves the device. adsbdb covers callsigns VRS lacks but its route data may not be stored or redistributed. Both need the plausibility check. |
| Arrival time | **Estimate** from remaining great-circle distance ÷ ground speed + ~15 min | none | No free keyless source exists; keyed APIs need a proxy and have tiny quotas. |
| Weather | **Open-Meteo** (free, non-commercial) | **MET Norway** Locationforecast | Open-Meteo is keyless, global, simple, and also returns the time zone. MET Norway allows commercial use and is the fallback, but its terms discourage direct client calls at scale and cap the whole app at 20 req/s. If a paid tier comes: Open-Meteo paid plan behind a proxy, or MET Norway behind a proxy. METAR from aviationweather.gov is an optional extra for observed conditions. |
| Airports | **VRS airports** (CC0, guaranteed to match VRS route codes) + **OurAirports** (public domain) for gaps | mwgg/Airports (MIT) | Both are free to bundle with no share-alike. Neither has a time zone column. |
| Time zone | **tzf-rs** at build time, giving a `tz` column in the bundled airport table | mwgg `tz`; Open-Meteo `timezone=auto` at runtime | Offline and accurate. ODbL attribution required; publish the generated table under ODbL to be safe. |

## Politeness defaults

| Setting | Proposal |
|---|---|
| Position poll | Every **15 s** while the view is visible; never under **10 s**; ±2 s jitter. Pause when hidden, backgrounded, minimised or the screen is locked (MET's terms require this for mobile; it is good manners everywhere). |
| Ambient mode (M4) | **60 s**, or 30 s at most, until a source agrees to more. At 15 s a full day is 5,760 requests, far above the 500/day adsb.lol header and OpenSky's 400 anonymous credits. |
| Radius | Default **20 nm (≈37 km)**; user range 5–25 nm. One request covers everything; never tile several requests. |
| Coordinates sent | Round to **2 decimals** (0.01° ≈ 1.1 km north–south, less east–west) before any request. Weather and route requests carry only destination airport data or callsigns, never the user's position. |
| Daily budget | Count requests per source per day on the device. Read `x-ratelimit-*` (adsb.lol) and `X-Rate-Limit-Remaining` (OpenSky). Switch to the fallback or slow to 60 s when below 10 % left. |
| Route cache | Bundled VRS snapshot; check for a new snapshot at most **once a day** with `If-None-Match`; publish a new one **weekly**. Online fallback (adsbdb): positive results **12 h** in memory only, negative results **6 h**, one lookup per new callsign, at most **10 lookups/min**. |
| Airport data | Bundled; refreshed with the route snapshot or the app release. No runtime calls. |
| Weather cache | Keyed by destination airport. Open-Meteo **15 min** (matches its 15 min `current` interval). MET Norway: honour `Expires` (≈30 min seen) and send `If-Modified-Since`. METAR **15 min**. Fetch only for flights on screen. |
| Timeouts | Connect 5 s, total 10 s. HTTPS only; accept gzip; follow redirects. |
| Backoff on 429 | Honour `Retry-After`, `X-Rate-Limit-Retry-After-Seconds` (OpenSky) or `x-ratelimit-reset` (adsb.lol); if absent, wait 60 s. |
| Backoff on 5xx / network | Exponential with full jitter: 15 s, 30 s, 60 s … capped at **10 min**. After **3** consecutive failures use the fallback; retry the primary every **5 min**. |
| On 403 / 401 | Do not retry in a loop (adsb.fi counts 4xx toward its limit and restricts IPs that send many bad requests). Disable that source for the session, log it, and show "source unavailable". |
| User-Agent | `PassingOver/<version> (+https://github.com/datstma/passingover)` on every request; the CLI uses `PassingOver-CLI/<version> (+https://github.com/datstma/passingover)`. This matches MET Norway's example form. No per-install or per-user identifiers. |
| Attribution screen | An "About data" view listing adsb.lol (ODbL), adsb.fi (link), Open-Meteo (link next to weather, as required), MET Norway, VRS standing data (CC0, credit as courtesy), OurAirports, timezone-boundary-builder / OpenStreetMap (ODbL). |

## Open questions / risks

1. **adsb.lol daily cap and future key.** The `500;w=86400` header would make
   a 15 s poll last about 2 h a day per IP. Ask the operator (info@adsb.lol)
   whether it is enforced per IP, whether a distributed app is welcome, and
   what the planned key means for apps. The key will need a proxy.
2. **adsb.fi is personal, non-commercial only.** Ask whether a free
   open-source app in app stores qualifies. It cannot stay as a source in a paid
   tier without an agreement.
3. **airplanes.live refused anonymous requests** (403, "contact us"). Its
   terms are unverified.
4. **OpenSky terms** appear to require a written agreement for a live product,
   and anonymous credits are small. The terms page could not be fetched
   directly; re-check in a browser.
5. **Shared IPs.** Mobile carrier NAT, offices and universities put many users
   behind one IP, so per-IP limits (adsb.fi 1/s, adsb.lol 500/day, Open-Meteo
   10k/day, OpenSky 400/day) may be shared. Handle 429 gracefully.
6. **Whole-app limits.** MET Norway caps all installations together at
   20 req/s. Open-Meteo applies limits "by IP addresses and application".
   Success at scale means a proxy.
7. **adsbdb route data rights.** Even caching may count as incorporating into
   another database. Ask before storing anything beyond a short in-memory
   cache.
8. **Route accuracy.** Callsign routes are crowd-sourced and can be wrong or
   stale. The plausibility check is required, and the UI should mark
   uncertainty.
9. **No arrival times.** "Landing in 7 h 40 m" will be an estimate.
10. **Commercial path.** A paid tier would need, at least, an Open-Meteo
    subscription (key, so proxy) or MET Norway via a proxy, agreements with
    adsb.lol (and adsb.fi or OpenSky if kept), and a check that adsbdb lookups
    are acceptable.
11. **ODbL duties.** adsb.lol data and the tzf time zone data need attribution;
    any database we publish that is derived from them (for example a shared
    route table learned from observations, or a proxy cache served to others)
    must be ODbL.
12. **Hobby-run services** (adsb.lol, adsb.fi, adsbdb, hexdb.io) can change
    terms or disappear. Keep `PositionSource`, `RouteSource` and
    `WeatherSource` pluggable and let a source be switched by configuration
    without a release.
13. **adsb.lol `routeset`** returned 201 with an empty body in three tries.
    Not needed if we bundle VRS.
14. **Unverified:** Open-Meteo prices; airplanes.live terms; OpenSky
    terms/FAQ wording; licensing of non-US METARs; where mwgg/Airports' records
    came from and TimeZoneDB/TimeAPI terms; which AeroDataBox endpoint gives
    arrival times.
15. **Things that will need a proxy later:** any keyed source (Open-Meteo paid,
    adsb.lol key, OpenSky OAuth secret, arrival-time APIs), shared caching to
    reduce load at scale, and hiding user IPs from providers (MET Norway
    recommends this).
