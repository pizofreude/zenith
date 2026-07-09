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

1. Match the brief to a row in `references/by-kind.md` (canvas, primitives, packs). Prefer the
   built-ins named there (`shape`/`chart`/`connector`, Lucide, flowchart/filters/masks packs) —
   do not fake them with plain rects.
2. If `.zenith/brand.md` (or a `libraries/*.zen` brand pack) exists in or above the working
   dir, read it and use its tokens. Otherwise scaffold with
   `zenith new <path> --theme <name>` plus canvas flags from the by-kind recipe
   (`zenith library list` / `zenith new --help`). Only invent a bespoke palette if no theme fits.
3. `zenith tokens <path>` — author only with those token ids (+ any extras you add). Write
   semantically-id'd nodes on the scaffolded page(s); add `styles` for shape labels.
4. `zenith validate <file> --json` — fix every hard diagnostic at the source.
5. `zenith render <file> --png <file>.png`, open the PNG, fix alignment/contrast if needed
   (`align_nodes` / `distribute_nodes` when siblings drift), then report the output path.

Do not invent syntax — verify with `zenith schema node <kind>` and `zenith <cmd> --help`.
