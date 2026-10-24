# Contributing

Thanks for looking. The project is tiny on purpose; keep it that way.

## Ground rules

- No runtime npm dependencies. Node built-ins only. The UI may load D3 from a CDN.
- Collectors must fail soft: if a tool is missing or a command errors, return an empty graph, never throw.
- Collectors must never read secret values. Variable names are fine, values are not.
- Keep the node type vocabulary in `src/graph.js` small. Prefer metadata on an existing type over a new type.

## Add a collector

1. Create `src/collectors/<name>.js` exporting `async function collect({ config, graph })`.
   `graph` is the merged graph so far (read-only by convention). Return a new `Graph`.
2. Add the name to `ORDER` in `src/scan.js` and to `collectors` in `stacklens.config.json`.
3. Run `node bin/stacklens.js scan` and check the per-collector line in the output.

## Add a framework or service detector

- Frameworks: add the package name to `NOTABLE` in `src/collectors/packages.js`.
- Services: add an env var prefix to `SERVICE_HINTS` in the same file.

## UI

Plain HTML, CSS and JS in `ui/`. Colors are CSS custom properties on `:root` with a dark mode block.
Node type colors come from a colorblind-checked categorical palette; keep the type-to-slot mapping stable.
