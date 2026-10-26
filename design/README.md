# Design

The Passing Over brand (D42) and the sources of the app icons (D43).

## Principles

- **Calm over busy.** Few elements on screen; generous space; slow, gentle motion.
- **Few numbers.** Destination, local time there and time to landing come
  first. Callsigns, altitudes and speeds stay in an opt-in "technical" layout.
- **Beautiful typography.** Type carries the design; no instrument-panel look.
- **Readable from across the room.** Ambient mode must work on a TV or a
  tablet on a shelf.
- **Metric by default**, imperial available.

## The icon

A jet seen from the ground at dusk: a slim lit fuselage, red and green
wingtip lights and twin contrails, on a sky fading from navy into peach.
There are no stars, which would read as a stargazing app, and no plane
silhouette seen from above, which every flight tracker uses.

`icon/` holds the masters, each drawn on 1024 units:

| File | What it is |
|---|---|
| `background.svg` | The dusk sky, full bleed |
| `foreground.svg` | Contrails, fuselage and lights. Everything that must stay visible lies within 313 units of the centre (Android's adaptive-icon safe zone). |
| `monochrome.svg` | One colour, for Android's themed icons; only alpha counts |
| `menubar.svg` | The macOS menu-bar template image, 18 × 18 pt |

After changing a master, run `cargo xtask icons` from the repo root. It
writes the icons for iOS, macOS, Android and Windows, and the menu-bar image,
into `app/`. Commit the result.

## Colour

The app lives in the night sky. The icon, website and store art use the hour
before it, dusk.

| Name | Value | Use |
|---|---|---|
| Deep night | `#03050C` | Ambient mode |
| Night | `#0B1426` | App background, top |
| Twilight | `#1C2B4A` | App background, bottom |
| Zenith | `#16244A` | Top of the dusk sky |
| Lavender | `#5A5C8C` | Middle of the dusk sky |
| Rose | `#E39A7A` | The glow above the horizon |
| Peach | `#F6C48C` | Horizon; the accent (11.6:1 on Night) |
| Port red | `#FF4D4D` | Left wingtip light |
| Starboard green | `#3DDC84` | Right wingtip light |
| LED amber | `#FFB000` | The LED ambient theme only |
| Paper | `#F5F3EE` | Light backgrounds: website, docs |
| Ink | `#0B1426` | Text on Paper (16:1) |

Text on the night sky is white at three strengths:

| Strength | Opacity | Use | Contrast on Night |
|---|---|---|---|
| Bright | 92% | Destinations | 15:1 |
| Soft | 70% | Details | 9.3:1 |
| Faint | 50% | Credits | 5.3:1 (4.7:1 on Twilight) |

## Type

[Overpass](https://github.com/RedHatOfficial/Overpass) 3.0.5, SIL Open Font
License 1.1. It descends from the US highway-sign alphabet. The app bundles
it at weights 200–700 in `app/assets/fonts/overpass/`, so it reads the same
on every platform. The large lines use ExtraLight and Light, details use
Light and Regular, and small labels use SemiBold with wide letter spacing.
The LED theme draws its own dot font (D37).
