# Project-library example

A self-contained project that defines its **own** library pack and materializes an item from it
into a document — no engine change, no rebuild.

```text
examples/library/
  libraries/
    studio-badges.zen     # the pack: id "@studio/badges", one component "pill" + its tokens/style
  poster.zen              # the target document (page id "page.poster")
```

The project directory is `poster.zen`'s parent (`examples/library/`); the resolver scans its
`libraries/*.zen` alongside the engine's embedded presets.

## List the resolved packs

```bash
zenith library list examples/library/
```

`@studio/badges` appears next to the built-in `@zenith/*` presets. (A project pack with the same
id as a preset would shadow it.)

## Add the badge to the poster

`library add` copies the component plus its token/style dependencies into the document and records
`libraries` + `provenance` entries — leaving a self-contained, deterministic `.zen`.

```bash
# Preview the resulting source without writing the file:
zenith library add @studio/badges#pill --into examples/library/poster.zen --page page.poster --at 40,40 --dry-run

# Write it in place:
zenith library add @studio/badges#pill --into examples/library/poster.zen --page page.poster --at 40,40

# Render the result:
zenith render examples/library/poster.zen --out .
```

Item spec is `<pack-id>#<item>` (here `@studio/badges#pill`). Optional flags: `--at X,Y` placement,
`--id <base>` to override the generated instance id, and `--dry-run` to print instead of write.

## Make it your own

Copy `libraries/studio-badges.zen`, change the `library id` self-entry and the `component`s, and
keep every token/style/asset the components reference declared in the same pack so materialized
items stay self-contained. See the repository [CONTRIBUTING.md](../../CONTRIBUTING.md) ("Libraries")
for the pack format and for contributing a built-in `@zenith/*` preset.
