# Contributing

Thanks for looking. The project is small on purpose; keep it that way.

## Ground rules

- Keep the dependency list short. Prefer std over a crate for small jobs.
- Collectors must fail soft: if a tool is missing or a command errors, return an empty graph, never panic.
- Collectors must never read secret values. Environment variable names are fine, values are not.
- Keep the `NodeType` vocabulary in `src/graph.rs` small. Prefer metadata on an existing type over a new type.

## Add a collector

1. Create `src/collectors/<name>.rs` with `pub fn collect(config: &Config, graph: &Graph) -> Graph`.
   `graph` is the merged graph so far (read-only). Return a new `Graph`.
2. Register it in `src/collectors/mod.rs`: add the module, add the name to `ORDER`, add a match arm in `run_one`.
3. Run `cargo run -- scan` and check the per-collector line on stderr.

## Add a framework or service detector

- Frameworks: add the package name to `NOTABLE` in `src/collectors/packages.rs`.
- Services: add an env var prefix to `SERVICE_HINTS` in the same file.

## UI

`src/ui/mod.rs` is the egui app; `src/ui/layout.rs` is the force layout. Node type colours come from a
colorblind-checked categorical palette with separate light and dark steps; keep the type-to-slot mapping stable.

## Release

```
cargo fmt && cargo clippy && cargo build --release
cargo publish --dry-run
```
