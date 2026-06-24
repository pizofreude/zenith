# Agentic visual workflow

Take a vague brief to a finished, auditable design without polluting the final file.

> Op fields: `zenith schema op <name>`, `zenith tx --help`, `examples/*.tx.json`.
> Attributes: `zenith schema node <kind>` / `zenith schema page`.

## 1. Capture the brief and plan

Record intent in the `agent-runs` block (`zenith inspect` surfaces it):

```kdl
agent-runs {
  run id="run.hero" brief="Launch hero: dark, energetic, product-forward" {
    step id="s1" action="generate-bg" action-version="1" {
      param name="palette" value="brand"
      param name="seed" value="7"
    }
  }
}
```

Steps carry action + params; the engine attaches affected node ids + diagnostics per step.

- Give each layer group a `semantic-role` (+ optional `layer-priority`/`intensity`) so layers stay
  addressable: `group id="bg.grunge" semantic-role="background" layer-priority=0`.
- Record acceptance criteria (e.g. "title contrast ≥ Lc 60 APCA"); check with `zenith validate`
  and the render.

## 2. Scratch experiments

Set `workspace-role="scratch"` on every experiment page; `finalize_run` acts on it later.

- Final pages: `page.<name>`. Experiments: `workspace-role="scratch"`.
- Set each experiment's `candidate-status` and `cleanup-policy` up front (step 3) so finalize is
  automatic (step 6).
- Scratch content reaches the deliverable only via promotion (step 5).

## 3. Generate candidates from one plan

- Create several candidate pages, each a different take on the same plan + tokens.
- Set `candidate-status` (`draft` → `selected`/`rejected`; other values fire
  `page.invalid_candidate_status`), `promotion-target` (the final page id), and `notes` /
  `cleanup-policy`.
- Keep all candidates on the same tokens so a palette change is one edit.

## 4. Render-preview and self-critique

```bash
zenith validate doc.zen --json              # hard diagnostics must be empty
zenith render doc.zen --all-pages preview/  # one PNG per page
```

- Treat every Error as blocking.
- Look at the PNGs: headline legible over the motif? product safe area clear? texture too noisy?
  Revise nodes by id and re-render.
- Record each preview in the `previews` block: one `preview` entry per `candidate` with
  `source-hash`, `output` + `output-hash`, `parent-revision` (provenance only; `zenith inspect`
  surfaces it).

## 5. Promote the chosen candidate

Set `candidate-status="selected"` on the winner, then:

```bash
# {"ops":[{"op":"promote_candidate","source_page":"page.scratch.hero.02","target_page":"page.hero","id_suffix":".final"}]}
zenith tx doc.zen promote.json --apply
```

Deep-copies the selected page into the target (content replaced), suffixing ids to stay unique.
Fields: `zenith schema op promote_candidate`. Then `validate` + `render`.

## 6. Finalize and clean up

```bash
# {"ops":[{"op":"finalize_run","run_pages":["page.scratch.hero.01","page.scratch.hero.02"]}]}
zenith tx doc.zen finalize.json --apply
```

For each rejected page, applies its `cleanup-policy`: `delete` removes the page; `archive` (or
absent) sets `workspace-role="archived"`. Fields: `zenith schema op finalize_run`. Then check
`zenith tokens <file>` for unused-token advisories; final source must validate + render clean.

## 7. History

```bash
zenith history doc.zen                      # list versions
zenith version doc.zen "v1-pre-promote"     # name a checkpoint
zenith undo doc.zen  /  zenith redo doc.zen
zenith restore doc.zen <rev>                # <rev> grammar: zenith restore --help
zenith sync doc.zen                         # capture an external/hand edit
```

Name a checkpoint before risky steps (e.g. promotion).

## 8. Later semantic edits

Stable ids + tokens + `semantic-role` groups make edits precise transactions:

- "Reduce the grunge" → `set_opacity` on `bg.grunge`.
- "Stronger glow" → update the shadow token it references.
- "Remove honeycomb near the headline" → delete/clip nodes in `bg.honeycomb`.

## Not implemented (don't assume these)

Brush/stamp definitions; an automated critique report (self-critique by reading the render, step 4).
