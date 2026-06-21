# grove — Phase 2: the WASM registry (multi-language)

Structural sight for coding agents. See [`VISION.md`](VISION.md) for the product
vision. This crate proves the model end to end: a real engine over six tools,
exposed through **two faces** that share one engine (a human CLI and an MCP
server), with grammars loaded **at runtime from a WASM registry** — so new
languages are added with no recompile and no toolchain on the consumer.

## Build

```bash
cargo build --release
```

The first build compiles wasmtime (~30s); after that it's incremental. Single
binary at `target/release/grove`. No grammar is compiled into the binary.

## Setup — `grove init`

One command makes a project one where the agent *uses* grove:

```bash
grove init           # in your project root
```

It detects which registered languages are present, then writes three things
(idempotently, preserving anything already there):

- **`.mcp.json`** — registers the grove MCP server (*availability* — the tools exist).
- **`CLAUDE.md`** — a steering directive in a marked section (*adoption* — the agent
  reaches for grove instead of grep/whole-file reads; see VISION §6.4.1).
- **`grove.lock`** — pins the detected grammars' version + wasm sha256.

`grove init --dry-run` detects without writing. Re-running only updates grove's
own pieces.

## Languages

Grammars live in a local registry stub (`registry/<lang>/` with `grammar.wasm`,
`tags.scm`, `manifest.json`) — the stand-in for the future hosted registry.

```bash
grove languages      # what's available
grove lock           # write grove.lock pinning version + wasm sha256
```

Currently registered: **rust**, **python**, **javascript**. Adding a language is
dropping a `registry/<lang>/` directory in — the binary doesn't change, and that
includes the *full* surface: the `manifest.json` carries a `profile` (container /
function / identifier node kinds) that drives `parent` grouping, `callers`'
enclosing-function, and go-to-def, so nothing language-specific is compiled in.

```jsonc
// registry/<lang>/manifest.json
{ "name": "go", "version": "…", "extensions": ["go"],
  "profile": {
    "function_kinds": ["function_declaration", "method_declaration"],
    "containers": [["type_declaration", "name"]],
    "identifier_kinds": ["identifier", "field_identifier"]
  } }
```

Build a grammar's wasm with `tree-sitter build --wasm` (emits the `dylink.0`
module the native runtime needs).

### Where grammars live

Grammars are a **cache** — reconstructible from the (future) hosted registry and
content-addressed by `grove.lock` — so the standard home is the OS-native cache
location. grove resolves the registry root by precedence (first existing wins);
`grove registry` shows it:

1. **`GROVE_REGISTRY`** env var — explicit override (CI, tests, air-gapped).
2. **`<project>/.grove/grammars/`** — project-vendored grammars (commit them for
   hermetic / offline builds), found by walking up from the cwd.
3. **OS user cache** — the default shared store:
   - Linux: `~/.cache/grove/grammars` (honors `$XDG_CACHE_HOME`)
   - macOS: `~/Library/Caches/grove/grammars`
   - Windows: `%LOCALAPPDATA%\grove\grammars`
4. **dev source tree** (`registry/` next to this crate) — only in a checkout.

Layout under the root is `<lang>/{grammar.wasm, tags.scm, manifest.json}`.

### Fetching grammars

`grove fetch` pulls grammars from the hosted registry into the OS cache:

```bash
grove fetch                 # all languages in the catalog
grove fetch python rust     # just these
grove fetch python --force  # re-download
```

The host is a **`grove-registry` GitHub repo served via jsDelivr's GitHub CDN**
(`cdn.jsdelivr.net/gh/<owner>/grove-registry@<tag>`) — CDN-backed, immutable by
tag, no rate limits, no infra to run. Layout: `<host>/index.json` (catalog of
name → version → wasm sha256) and `<host>/<lang>/{grammar.wasm, tags.scm,
manifest.json}`. Each wasm's sha256 is verified against the catalog before it's
written, so a corrupted or tampered grammar is rejected. Override the host with
`GROVE_REGISTRY_URL` (self-host, fork, or a local mirror).

## Tools (the agent loop, in miniature)

| Phase | Command | What it does |
|---|---|---|
| Read | `grove outline <file> [--kind K] [--detail 0\|1\|2]` | compact definition skeleton: kind · name · parent · signature · id. Filter by kind / dial detail down for big files |
| Find | `grove symbols <dir> [--kind K] [--name SUB] [--refs]` | repo-wide symbol search (gitignore-aware) |
| Read | `grove source <id>` or `grove source <file> <name>` | full source of one symbol — no whole-file read |
| Verify | `grove check <file>` | ERROR / MISSING nodes (exit 1 if any) — post-edit syntax check |
| Trace | `grove callers <name> [-d <dir>]` | call sites of a symbol, each with its enclosing function |
| Trace | `grove definition <name> [-d <dir>]` or `grove definition --at <file:row:col>` | go-to-def, by name or from a usage position |

Add `--json` to any command for the agent-facing structured shape. Every result
carries a stable `symbol-id` (`<lang>:<relpath>#<name>@<row>`) usable across turns.

## Examples

```bash
grove languages
grove outline foo.py --kind class        # python, loaded from wasm at runtime
grove outline src/engine.rs --kind function
grove source  src/mcp.rs serve
grove callers extract -d src
grove check   src/registry.rs
```

## The MCP face (for agents)

`grove serve` runs an MCP server over stdio (newline-delimited JSON-RPC 2.0),
exposing the same six tools for every registered language. Register it with a
coding agent and the agent gains structural sight.

Project-scoped registration for Claude Code lives in [`.mcp.json`](.mcp.json):

```json
{ "mcpServers": { "grove": { "command": "…/target/release/grove", "args": ["serve"] } } }
```

A `CLAUDE.md` steering directive (see VISION §6.4.1) is what gets the agent to
actually *use* the tools rather than defaulting to grep/whole-file reads.

Tool results are JSON inside an MCP text block; tool-level failures come back as
`isError: true` with a message so the model can recover.

## What this slice deliberately is not

- **Local registry stub, not the hosted registry.** Grammars are real wasm loaded
  at runtime, but served from a local directory; hosting + signing is still ahead.
- **No staleness/incremental, no `map`/`grep`/`ast` yet.** Those are next. `callers`
  and `definition` are name-based (no receiver-type / local-scope resolution).

## Layout

```
registry/<lang>/   grammar.wasm + tags.scm + manifest.json (the registry stub)
src/main.rs        CLI dispatch (clap) — six verbs + init/languages/lock/serve
src/init.rs        `grove init` — detect languages, write .mcp.json + CLAUDE.md + lock
src/fetch.rs       `grove fetch` — download grammars from the hosted registry (GitHub/CDN)
src/registry.rs    grammar resolver, extension map, lockfile — the registry spine
src/ops.rs         the operations as a library — the shared engine both faces call
src/mcp.rs         MCP server — newline-delimited JSON-RPC over stdio
src/engine.rs      wasm load + Query-based tags, source slicing, check, position resolution
                   (node-kind profiles are data — they come from each manifest, not code)
```
