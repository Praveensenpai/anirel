# anirel

**Find the related anime you have not watched yet.**

`anirel` maps your MyAnimeList library against the *Related Entries* graph and
tells you which seasons, OVAs, films and spin-offs you are missing — with a
watch order, cover art, and without the noise.

```
$ anirel related "Yuru Camp"

Yuru Camp△ — 6 of 10 titles owned, 4 missing, 1 unwatched

  ✓ Laid-Back Camp                                TV · 12 ep · 2018 · score 10
      completed · other
  ✓ Yuru Camp△ OVA                                OVA · 3 ep · 2018 · score 10
      completed · side story
  ◌ ROOM CAMP                                     TV_SHORT · 12 ep · 2020
      MISSING · side story
  ◌ ROOM CAMP: Saunas and Grub and Three-Wheele…  OVA · 1 ep · 2020
      MISSING · side story
  ✓ LAID-BACK CAMP SEASON2                        TV · 13 ep · 2021 · score 10
      completed · sequel
  ◌ Yuru Camp△ SEASON 2 OVA                       OVA · 2 ep · 2021
      MISSING · side story
  ✓ Laid-Back Camp The Movie                      MOVIE · 1 ep · 2022 · score 9
      completed · sequel
  ✓ Laid-Back Camp Season 3                       TV · 12 ep · 2024 · score 10
      completed · sequel
  ○ Laid-Back Camp Season 3 OVA                   OVA · 3 ep · 2024
      plan-to-watch · side story
  ◌ Yuru Camp△ SEASON 4                           TV · ? · 2027 · not aired
      MISSING · sequel
```

---

## Install

```bash
git clone <this repo> && cd anirel
cargo install --path .
```

Requires Rust 1.85 or newer.

## Quick start

```bash
anirel                          # open the interactive interface
anirel related "Yuru Camp"      # everything related to one title
anirel gaps --user YOUR_NAME    # every missing related entry
anirel export --user YOUR_NAME --format csv -o gaps.csv
anirel search "yuru camp"       # find titles by name
```

The username is remembered after the first run, so plain `anirel` is enough
afterwards. `ANIREL_USER` works too.

## The interactive interface

Running `anirel` with a terminal attached opens the TUI.

```
┌ anirel · Praveen20 ────────────────────────────────── ● 1501 titles ────────┐
│ ▸ Library 1501     Missing 5                          ⌕ press / to filter      │
├──────────────────────────────┬──────────────────────────┬──────────────────┤
│ ▌ Laid-Back Camp         ✓ 10 │ 6/10 owned · 4 missing    │ ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀ │
│   Laid-Back Camp Season 2 ✓10 │                          │ ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀ │
│   Laid-Back Camp Season 3 ✓10 │ ▌ ✓ Laid-Back Camp        │ ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀ │
│   …                            │   completed · other      │ ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀ │
│                              │   ✓ Yuru Camp△ OVA        │                   │
│                              │     completed · side story│ Laid-Back Camp    │
│                              │   ◌ ROOM CAMP            │ TV · 12 ep · 2018 │
│                              │     missing · side story  │                   │
│                              │   ✓ LAID-BACK CAMP SEASON2│ ▰▰▰▰▰▰▰▰▰▰▰▰▱▱▱  │
│                              │     completed · sequel    │ 6/10              │
│                              │   ◌ ROOM CAMP: Saunas…    │ 4 missing         │
├──────────────────────────────┴──────────────────────────┴──────────────────┤
│  tab focus  / filter  L load  o open  c copy  a scan all  e export  ? help  │
└──────────────────────────────────────────────────────────────────────────────┘
```

Three panes: **your library**, **the franchise of the selected title**, and a
**poster card** with the cover art, the facts and a completion bar.

| Key | Action |
|---|---|
| `↑` `↓` / `j` `k` | move the selection |
| `tab` / `shift+tab` | switch focus between the panes |
| `/` | filter (matches the original *and* the English title) |
| `1` / `2` | library view / missing view |
| `L` | **load the franchise of the highlighted title** |
| `o` / `enter` | open the title on MyAnimeList |
| `c` | copy the MyAnimeList link |
| `a` | load every franchise at once (full scan, with progress) |
| `e` | write a Markdown report to the cache directory |
| `r` | clear the cache and rescan |
| `?` | help overlay |
| `q` / `ctrl+c` | quit |

### Lazy by default

Nothing is fetched until you ask for it. The library comes from MyAnimeList
(6 pages, a couple of seconds); the relation graph is only pulled when you
press `L`. That is a handful of requests instead of 1,500, and the interface
is usable immediately.

Pressing `L` on a **related** title inside a loaded franchise dives into
*that* title's own franchise - from Yuru Camp into Heya Camp's subtree - and
`o` opens whichever title is under the cursor.

`a` still offers the full sweep in the background with a progress bar, for
when you want the global missing list.

## Cover art

Posters are rendered from the real cover image:

- **Half-block art** (default) - the JPEG is decoded, downscaled and drawn as
  `▀` cells, upper pixel in the foreground colour and lower pixel in the
  background. Works in every colour terminal; truecolor is used when
  `COLORTERM` allows it, otherwise the 256-colour palette.
- **Native graphics** - kitty and iTerm2 are detected automatically and can
  show the image untouched.

Covers are cached under `$XDG_CACHE_HOME/anirel/covers`.

## How it works

| Data | Source | Notes |
|---|---|---|
| Your library | `myanimelist.net/animelist/<user>` | Public list pages, paginated. Status, score, episodes watched and cover art included. |
| Relations | AniList GraphQL, keyed by MyAnimeList id | Batched 20 ids per request, so 1,500 titles cost ~75 requests instead of 1,500. |
| MAL parity | `myanimelist.net/anime/<id>` | `anirel related` also reads MyAnimeList's own *Related Entries* and merges in anything the graph missed. |

AniList's relation graph is a superset of MAL's *Related Entries* for most
franchises, and it also resolves manga ids correctly (which MAL's web pages
cannot).

**Relation kinds that count as a gap:** prequel, sequel, parent story, side
story, spin-off, summary, alternative version, and crossovers
(*character(s) in*). Manga adaptations are excluded, and so are the vague
"other" links — those are context, not something to watch. The single-title
view (`anirel related`) keeps them, because that is exactly what MyAnimeList
lists under *Related Entries*.

**Watch order** is a topological sort over the relation edges, with release
year, format and title as tie-breakers — so side stories land after the
season they belong to, and films land after the season they continue from.
A franchise is named after the title the rest of it hangs off, so
`Yuru Camp△` stays the name of its franchise.

### Cache

Everything is cached under `$XDG_CACHE_HOME/anirel` (usually
`~/.cache/anirel`):

| Path | Contents | Lifetime |
|---|---|---|
| `library/<user>.json` | your list | 6 hours |
| `media/<id>.json` | resolved titles and relations | 30 days |
| `covers/<hash>.jpg` | cover images | forever |
| `report.md` | last exported report | — |

`--refresh` ignores the cache; `r` in the TUI clears the media cache and
rescans.

## Terminal requirements

Required: a TTY, UTF-8, and at least 64×16 cells. Everything else is an
enhancement — truecolor, native graphics and the OSC 52 clipboard are used
when available. The poster pane appears from 96 columns upwards; below that
the interface collapses to two panes. The plain CLI output is used
automatically when stdout is not a terminal, so `anirel gaps | less` and
`anirel gaps --json | jq` both work.

`NO_COLOR` and `TERM=dumb` are respected.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | success |
| `1` | runtime failure (network, parse, unknown user) |
| `2` | usage error (missing username or arguments) |

## Development

```bash
cargo test          # unit, integration and TUI snapshot tests
cargo clippy --all-targets -- -D warnings
cargo fmt
cargo run --example render -- 140 30   # print the TUI without a terminal
```

The live network checks are ignored by default:

```bash
cargo test -- --ignored --nocapture
```

### Layout

```
src/
  domain/     models, relation kinds, franchise clustering, watch order
  infra/      MyAnimeList web, AniList GraphQL, disk cache
  app/        state, key mapping, pure updates, background scanner
  ui/         Ratatui rendering: chrome, panes, poster, widgets, theme, help
  art.rs      cover art: fetching, caching, half-block rendering
  engine.rs   library loading, batched resolution, expansion
  commands.rs non-interactive commands
  tui.rs      interactive runner
examples/
  render.rs   render the TUI to a string without a terminal
```

## Notes and limits

- `anirel` reads public pages. It is polite: one request per 2.2 s against
  AniList, and 0.6 s between MyAnimeList pages.
- MyAnimeList ids that AniList does not know are skipped instead of failing
  the batch.
- Titles that only exist on MAL (rare, but some entries are MAL-only) are
  still reachable through the MAL deep pass in `anirel related`.
- No streaming-provider data is fetched. It is the most requested feature and
  the most fragile one to keep correct.
