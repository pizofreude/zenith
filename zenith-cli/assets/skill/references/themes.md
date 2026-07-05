# Themes — token contract & catalog

A **theme** is a complete set of design tokens (a palette _and_ a shape language) under a fixed
id contract, so any document built on the contract can be re-skinned by swapping the theme.
Themes ship as 10 embedded packs (`@zenith/theme.<name>`) built into the CLI — no files to
find or copy; reach them via `zenith new --theme <name>` and `zenith theme apply <name> <doc>`.

> Provenance: these are converted from daisyUI themes. daisyUI uses `oklch`; **Zenith colors
> are sRGB hex / CMYK only**, so values are converted to hex. A theme is a full contract, so a
> document rarely uses every member — that rolls up into one `token.set_partially_used`
> advisory (not an error) instead of per-token noise; see "Using a theme" below.

## The contract (token ids every theme defines)

**Colors** — each role pairs with its readable foreground (`*.content`), which makes on-brand
text contrast-safe by construction:

- Surfaces: `color.base.100` (page), `color.base.200`, `color.base.300`, `color.base.content` (text on base)
- Roles: `color.primary` + `.content`, `color.secondary` + `.content`, `color.accent` + `.content`, `color.neutral` + `.content`
- Status: `color.info` + `.content`, `color.success` + `.content`, `color.warning` + `.content`, `color.error` + `.content`

**Shape** (this is what differentiates themes beyond color):

- `radius.box` (cards/frames), `radius.field` (buttons/inputs), `radius.selector` (badges/toggles)
- `border.width` (default stroke width), `space.unit` (base spacing step)
- `shadow.depth` (a soft elevation shadow) — present only when the theme has depth; flat themes omit it
- _noise_ — a flag in the file header; `1` ⇒ apply a grain-overlay (a `noise` filter kind inside a `filter` token — see `zenith schema token filter`)

**Type** (added — daisyUI omits type):

- `font.heading`, `font.body` (both Noto Sans), `size.h1` 64, `size.h2` 40, `size.body` 28, `size.caption` 18 (px)

## Catalog

Light themes:

| Theme    | Character                     | base.100  | primary   | accent    | box radius | depth/noise       |
| -------- | ----------------------------- | --------- | --------- | --------- | ---------- | ----------------- |
| `prism`  | bright cyan/violet, raised    | `#f8f8f8` | `#00d0ef` | `#7c85ff` | 8px        | depth 1 · noise 1 |
| `sorbet` | soft warm pastel, rounded     | `#f8f8f8` | `#ffb667` | `#8bc2ff` | 16px       | flat              |
| `cobalt` | crisp corporate indigo        | `#f7f9fa` | `#605dff` | `#00a4f2` | 32px       | noise 1           |
| `volt`   | electric lime + black, punchy | `#f8f8f8` | `#b9f14e` | `#000000` | 32px       | depth 1           |
| `poppy`  | vivid scarlet + magenta       | `#f7f9fa` | `#f82834` | `#f43098` | 16px       | depth 1 · noise 1 |
| `lagoon` | teal + blue, crisp/technical  | `#f7f8fa` | `#009689` | `#135bf9` | 4px        | depth 1 · noise 1 |

Dark themes:

| Theme    | Character                   | base.100  | primary   | accent    | box radius | depth/noise          |
| -------- | --------------------------- | --------- | --------- | --------- | ---------- | -------------------- |
| `pine`   | emerald/teal, minimal flat  | `#030712` | `#00d390` | `#979fad` | 8px        | flat                 |
| `ember`  | warm amber-gold + green     | `#0b0908` | `#fcb700` | `#99e600` | 32px       | depth 1 · noise 1    |
| `harbor` | navy, warm amber + sky      | `#010515` | `#ffb667` | `#71d1fe` | 16px       | noise 1 (2px border) |
| `sunset` | navy, orange/amber + indigo | `#010515` | `#ff8904` | `#7c85ff` | 4px        | noise 1              |

Light/dark pairing suggestions: `sorbet`↔`pine`, `cobalt`↔`harbor`, `prism`↔`sunset`.

(More themes are added over time; run `zenith library list` to see the current set. To generate a
theme from brand colors — logo, website, or brand docs — run `zenith theme new --help`.)

## Using a theme

**Start a new document on a theme:**

```bash
zenith new doc.zen --theme sunset
```

This scaffolds the full contract below into `tokens { … }` and points the page background at
`color.base.100`, so every node you add can build on the role tokens directly:

```kdl
rect id="card" x=(px)80 y=(px)120 w=(px)600 h=(px)360 fill=(token)"color.base.200" radius=(token)"radius.box" shadow=(token)"shadow.depth"
text id="title" x=(px)112 y=(px)152 w=(px)536 h=(px)80 fill=(token)"color.base.content" font-family=(token)"font.heading" font-size=(token)"size.h1" { span "On-theme" }
rect id="cta"  x=(px)112 y=(px)400 w=(px)220 h=(px)64 fill=(token)"color.primary" radius=(token)"radius.field"
text id="cta.t" x=(px)112 y=(px)416 w=(px)220 h=(px)40 fill=(token)"color.primary.content" font-family=(token)"font.body" font-size=(token)"size.body" { span "Get started" }
```

Because everything references the contract, putting `color.primary` text on `color.primary`
fill uses `color.primary.content` — contrast handled by the theme.

**Re-skin or switch an existing document's theme:**

```bash
zenith theme apply cobalt doc.zen           # preview the token diff (dry-run)
zenith theme apply cobalt doc.zen --apply   # write it
```

`<pack>` is a bare embedded theme name or a full pack id (`@zenith/theme.cobalt`); it works for
any pack that carries a `tokens` block, project or embedded (`zenith library list`). Switching
light/dark, or between any two themes, is one `theme apply` away — no manual token editing.

**Provenance:** tokens written by a theme carry `set="@zenith/theme.<name>"`. A theme file is a
full contract, so a given document rarely uses every member; `validate` rolls those up into one
`token.set_partially_used` advisory per set instead of a `token.unused` warning per token. Don't
trim the contract down to just what's referenced — keeping it whole is what makes re-skinning a
single `theme apply` instead of a hand-edit.

## Light / dark

Zenith has no light/dark _mode_ — light↔dark is just choosing a light theme vs a dark one (or
swapping the token values). Pair a light theme with a dark one for the two modes, e.g.
`sorbet`/`pine`. Because every role has a `.content` pair, text stays readable in both.

## Project default

To make a theme the project default, note it in `.zenith/brand.md` (`references/brand.md`) so
new documents start with `zenith new --theme <name>` and existing ones get `theme apply <name>`.
Don't rely on an engine default — Zenith intentionally ships only default _fonts_, not a
default palette; themes are explicit.
