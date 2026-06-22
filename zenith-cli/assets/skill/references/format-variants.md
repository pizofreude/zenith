# Size / format variants (`zenith variant`)

Turn **one canonical page into many named target sizes** — square, story, banner, ad slots —
deterministically. This varies **dimensions** (and small per-target tweaks). It is distinct from
`zenith merge`, which varies **content** across CSV rows (see `references/variants.md`). Reach for
`variant` for "the same design at 4 sizes"; reach for `merge` for "this design for 200 people".

## The `variants` block

Declare it at the document level (additive — absent means byte-identical output). Each `variant`
names a `source` (the canonical page id) and the target `w`/`h`, with optional per-node `override`s:

```kdl
variants {
  variant id="square" source="page.main" w=(px)1080 h=(px)1080 {
    override node="qr"       visible=#false
    override node="legal"    text="© 2026 Acme"
    override node="headline" fill=(token)"color.brand"
  }
  variant id="story" source="page.main" w=(px)1080 h=(px)1920 {
  }
}
```

- `override` props map 1:1 to typed transaction ops and are intentionally small:
  - `visible=#true|#false` — show/hide the target node
  - `text="…"` — replace a text node's content
  - `fill=(token)"…"` — re-fill (token id only; Zenith is token-first)
- A variant with no overrides (`story` above) is just a resize.
- Generated page ids are deterministic: `<source>.<variant-id>`.

## The command

```bash
zenith variant <doc.zen> --out-dir out/ [--json] [--manifest manifest.json]
```

- `--out-dir <DIR>` (required) — writes `<stem>-<id>.zen` (a native, reviewable page) **and**
  `<stem>-<id>.png` per variant.
- `--json` — machine-readable batch report (per-variant provenance).
- `--manifest <PATH>` — deterministic generation manifest (`schema: zenith-variant-manifest-v1`):
  SHA-256 of the input, targets in id order, no absolute paths or timestamps → **byte-reproducible**
  in CI.

## Why it's reliable

- **Token propagation is free.** Variants are overrides *on* the canonical page, so they inherit
  the source tokens — change a brand token once and every size re-renders on-brand.
- **Anchored nodes reflow.** Use `anchor` / `anchor-zone` (see `references/layout.md`) so logos,
  CTAs, and page numbers stay correctly placed at every size; only free-coordinate decorative
  nodes need per-variant repositioning.
- **Deterministic.** Same source → byte-identical `.zen`, PNG, and manifest across runs.

## Workflow

1. Build and `zenith validate` the canonical page first — a broken source fails every variant.
   Variant-specific diagnostics: `variant.duplicate_id`, `variant.unknown_source`,
   `variant.invalid_dimension` (non-px or ≤ 0), `variant.override_unknown_node`.
2. Generate, then open a couple of the PNGs to eyeball reflow at the widest/tallest sizes.
3. For CI, pass `--manifest` and commit it so the batch is auditable and reproducible.

Run `zenith variant --help` for exact flags.
