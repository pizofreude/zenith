---
description: Scaffold a new .zen design document from a brief (validates + renders a first preview).
argument-hint: "[brief, e.g. 'square instagram promo for a coffee launch']"
allowed-tools:
  - Bash(zenith:*)
  - Read
  - Write
  - Glob
---

Create a new Zenith design document for: **$ARGUMENTS**

Follow the `zenith` skill. Steps:

1. If `.zenith/brand.md` (or a `libraries/*.zen` brand pack) exists in or above the working
   dir, read it and use its tokens. Otherwise pick a fitting embedded theme and scaffold with
   `zenith new <path> --theme <name>` (`zenith library list` shows the 10 embedded themes).
   Only invent a bespoke palette if the brief explicitly demands a look no theme covers.
2. Choose a canvas size from the brief (e.g. 1080×1080 social square, 1080×1920 story,
   poster). Start from a `templates/` starter if one fits.
3. Write the `.zen` source: a `document` with a `page` and semantically-id'd nodes, building on
   the tokens already scaffolded by the brand/theme (or the bespoke palette from step 1) — add
   only doc-specific token extras. Capture the brief in a `note`.
4. `zenith validate <file> --json` — fix every hard diagnostic at the source.
5. `zenith render <file> --png <file>.png`, then describe what you produced and the output path.

Do not invent syntax — verify with `zenith schema node <kind>` and `zenith <cmd> --help`, and
mirror the skill's `templates/*.zen` starters.
