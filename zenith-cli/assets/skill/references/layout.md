# Layout — pages, anchors, frames, spreads

## Pages

```kdl
document id="doc.social" title="Social" {
  page id="page.sq" w=(px)1080 h=(px)1080 background=(token)"color.bg" {
    # nodes…
  }
}
```

- `w` / `h` set the canvas in px; `background` takes a color **or** gradient token.
- A document can hold multiple pages (deck slides, book pages, size variants). Render one with
  `--page N`, all with `--all-pages <dir>`, or a facing-page `--spread A-B`.

## Coordinates vs anchors

Nodes take explicit `x y w h`. Instead of hand-computing position, set `anchor` to a nine-point
name and let it resolve to deterministic geometry (identical bytes to the hand-placed version).
An explicitly-authored `x` or `y` always wins over the anchor-derived value **per axis**, and the
node's `w`/`h` must be present (px) for derivation.

Nine-point names (see `examples/anchors.zen`):
`top-left top-center top-right center-left center center-right bottom-left bottom-center bottom-right`.

```kdl
rect id="logo" w=(px)160 h=(px)60 fill=(token)"color.brand" anchor="top-left"
text id="cta"  w=(px)300 h=(px)80  anchor="bottom-right" font-size=(token)"size.body" { span "Buy now" }
```

### Reference frame: page, safe-zone, parent, or sibling

The nine-point name says *which corner*; a second attribute chooses *what it's relative to*.
Default is the page. Precedence when more than one is present: **zone > sibling > parent > page**.

- **Page** (default) — `anchor="bottom-right"` → relative to the page box.
- **Safe-zone** — declare a `safe-zone` in the page, then anchor into it:
  ```kdl
  page id="page.main" w=(px)1080 h=(px)1080 {
    safe-zone id="sz.body" type="required" x=(px)80 y=(px)80 w=(px)920 h=(px)920
    text id="cta" w=(px)300 h=(px)80 anchor="bottom-right" anchor-zone="sz.body" { span "Buy now" }
  }
  ```
  Keeps content off the bleed/margins — anchor relative to the safe area, not the page edge.
- **Parent container** — `anchor-parent=#true` anchors within the node's direct `frame`/`group`
  box instead of the page (e.g. a label pinned to the corner of its card).
- **Sibling** — `anchor-sibling="<id>"` anchors relative to a sibling node's box in the same
  container (e.g. a badge clinging to the top-right of a logo). The sibling must be an
  anchor-bearing node with known `w`/`h`; cycles are rejected (`anchor.cycle`).

All four resolve to explicit, deterministic geometry at compile time (absent anchor = byte-identical
to hand-placed coords). Use them for logos, page numbers, captions, badges, and CTAs so they stay
correctly placed across size variants (see `references/format-variants.md`).

## Frames (clipping) and groups

- `frame id x y w h { … }` clips its children to its box — use it for image windows, cards,
  and any "nothing escapes this region" layout (`examples/frame.zen`).
- `group id { … }` bundles nodes logically (no clip) so a whole motif moves/dims/deletes as a
  unit (`examples/group.zen`). Opacity and transforms cascade through groups/frames.

## Dividers and rules

`line x1 y1 x2 y2 stroke=(token) stroke-width=(token)` for separators/rules
(`examples/line.zen`).

## Multi-size variants

For square/story/banner from one design, **declare a `variants` block and run `zenith variant`**
— a first-class, deterministic regeneration model (see `references/format-variants.md`). It
expands one canonical page into N named target sizes, with per-variant `override`s and automatic
token propagation. Anchored nodes reflow to each size; reposition free-coordinate decorative nodes
via overrides. (This is size/format variation — for varying *content* across data rows, that's
`zenith merge`; see `references/variants.md`.)

## Always verify

Anchors, frames, and clipping interact; render (`--all-pages` for a contact sheet) and look
before finalizing, and `zenith validate` to catch off-canvas / overflow.
