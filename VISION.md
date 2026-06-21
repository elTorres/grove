# grove — Product Vision

> *Structural sight for coding agents. One command to give any agent AST-aware
> tools for any codebase, with no toolchain to install.*

**Status:** Vision / pre-build · **Owner:** Boni Gopalan · **Date:** 2026-06-21
**Name:** `grove` is a working placeholder (tree-sitter → a grove of trees). Rename later.

---

## 1. The one-liner

`grove` is a CLI a developer runs **once** to give their coding agent (Claude
Code, Codex, antigravity, pi, …) structural sight into a codebase. After that it
is invisible: the agent simply *has* AST tools the way it has file-read.

---

## 2. Why now — the problem

A coding agent's most expensive failure mode is **reading whole files to answer
structural questions**:

- "Where is `parse_config` defined?"
- "What calls this function?"
- "Give me the exact span of the method at line 88 so I can edit it."
- "Find every `await` that isn't inside a `try`."

Today the agent answers these by dumping files into context and pattern-matching
in its head, or by regex-grepping — which is token-hungry, unreliable on large
files, and blind to syntax. The structural information it needs already exists in
a parse tree; nothing puts that tree in front of the agent in a form it can cheaply
consume.

Tree-sitter is the right engine for this — fast, incremental, error-tolerant,
dependency-free, and already emitting **structured data with byte + row/col ranges**
rather than prose. What's missing is the **distribution, registration, and
agent-ergonomics layer** that turns that engine into a tool an agent picks up
automatically. That layer is `grove`.

---

## 3. The core product insight

There are **two users with opposite needs**, and the UX must serve both from one
engine:

- **The developer** wants setup that disappears — one command, then silence.
- **The agent** wants tools that are token-cheap, self-describing, and return
  **stable addresses** it can reference across turns (so isolated tool calls
  become a coherent read → locate → edit loop).

Every design decision below follows from holding both of these at once.

---

## 4. Goals & non-goals

### Goals
- **One-command setup.** The developer's entire conscious interaction is
  `grove init`. Languages, agent type, and grammars are auto-detected.
- **Toolchain-free on the consumer.** Runs inside any agent sandbox with no
  compiler, runtime, or language toolchain present.
- **Token-frugal, structured tool outputs.** Bounded by default; the agent opts
  into verbosity only when it needs to learn node shapes.
- **Stable addresses across the loop.** Symbol-ids and byte ranges the agent can
  carry forward between turns.
- **Reproducible.** A committed lockfile pins grammar versions + content hashes so
  every machine, teammate, and CI run parses identically.
- **Read-only / locate-don't-mutate.** `grove` returns precise locations; the
  agent's own Edit tool performs writes. Keeps the trust and permission story
  trivial and composes with any harness.

### Non-goals (at least for v1)
- Not a refactoring engine. No rename/move/extract tools — `grove` locates, the
  agent edits.
- Not a grammar authoring tool. Agents *consume* grammars; they don't write them.
  The tree-sitter generator is explicitly out of scope.
- Not a renderer. No terminal/HTML syntax highlighting — agents don't render.
- Not a language server. No diagnostics, completion, or type inference.

---

## 5. The two users

### 5.1 The developer
Runs `grove init` in a project and never thinks about it again. May occasionally
run a CLI verb (`grove outline src/server.py`) to see exactly what the agent sees —
the human-debuggable symmetry that builds trust.

### 5.2 The agent
Calls `grove`'s tools (via MCP or CLI) to navigate code structurally. Sees tool
descriptions written *for an LLM reader*, gets token-bounded structured results,
and receives errors that teach (a bad query returns the syntax error **plus** the
valid node kinds near the offset, so it self-corrects in one turn).

---

## 6. The product experience

### 6.1 Install — one static binary, every channel
```
brew install grove
npm i -g @grove/cli
curl -fsSL grove.dev/install | sh
```
Single self-contained binary. No runtime, no toolchain. Grammars are WASM, loaded
by an embedded engine, so the host needs nothing to compile. This matters because
many installs happen inside an agent's sandbox where no compiler exists.

### 6.2 The hero flow — `grove init`
The whole product is one command:
```
$ grove init

  grove  scanning ./

  detected   python ·············· 142 files
             typescript ·········· 88 files
             rust ················ 12 files
  resolving  3 grammars … done (cached, 0 downloaded)

  agent      detected Claude Code  (.claude/ present)
             register grove as an MCP tool source? [Y/n] y
             ✓ wrote .mcp.json            (availability — the tools exist)
             ✓ wrote CLAUDE.md            (adoption — steer the agent to them)
             ✓ added grove.lock

  ready      your agent now has AST tools across its whole loop:
             map · outline · symbols · query · grep · source
             locate · callers · definition · check · ast

  try it     ask your agent:  "outline src/server.py"
```

Deliberate UX choices:
- **Auto-detect, don't ask.** Languages from extensions + content sniffing.
  Overridable (`--only python,rust`) but never required.
- **Grammars resolve silently** into the OS-native cache (`~/.cache/grove/grammars`
  on Linux, `~/Library/Caches/grove/grammars` on macOS, `%LOCALAPPDATA%\grove` on
  Windows), shared across every project. The second project in a language is instant.
  Resolution precedence: `GROVE_REGISTRY` → project `.grove/grammars/` → OS cache.
- **Self-registration is the magic moment — but it has two halves.** `grove` detects
  *which agent* lives in the repo and writes the correct files. The developer never
  hand-edits MCP JSON — the #1 friction point in agent-tool products today is deleted.
- **Registration buys availability; a directive buys adoption.** *This is a field
  learning, not a guess* (see §6.4.1). Writing `.mcp.json` makes the tools *present*,
  but a cold agent still defaults to grep / whole-file reads because (a) nothing tells
  it to prefer grove, and (b) MCP tools arrive deferred — it must pay a schema-load
  step before the first call. So `init` also writes a **steering directive** into the
  agent's instruction file (`CLAUDE.md`, `AGENTS.md`, `.cursorrules`, …): "for any
  where/what/who-calls/what's-in-this-file question, use grove before grepping or
  reading whole files; run `check` after edits." That directive is what makes the
  agent load the schemas up front instead of reaching for its default.
- **It tells the agent what to do.** The closing block is copy-paste guidance, and
  the tool descriptions themselves coach the agent so the developer doesn't have to.

**Zero-config escape hatch:** skip `init` and the agent can still call
`grove outline foo.py` directly; grammars lazy-resolve on first use. `init` just
makes it persistent and registered.

### 6.3 The agent-facing tool surface

The tool surface is organized around the **six phases of an agent's working loop**
on a codebase — orient, find, read, locate, trace, verify — plus a learning aid.
Each tool is token-bounded by default and returns **stable addresses**, not just text.

| Phase | Tool | Agent asks | Returns |
|---|---|---|---|
| **Orient** | `map` | "I've never seen this repo — show me its shape" | tiered repo tree with per-file symbol summaries (detail 0–3), bounded. The cold-start move. |
| **Find** | `symbols` | "where is `X` defined / used?" | repo-wide defs + refs, each with a stable `symbol-id` and byte range. |
| | `query` | structural search regex can't do | matches with capture name, node kind, text, byte + row/col range. |
| | `grep` | literal / regex text search | matches with ranges; **scope-aware** mode skips comments and strings (only an AST tool can do this). |
| **Read** | `outline` | "what's in this file?" | compact def list: `kind · name · signature · range · symbol-id`. A 2000-line file → ~300 tokens. |
| | `source` | "give me the exact code of this symbol" | full source of a `symbol-id` (no whole-file round-trip), tiered. |
| **Locate** | `locate` | "exact span of the function at line 88" | smallest enclosing named node + ancestor chain + **byte range to edit**. |
| **Trace** | `callers` | "what calls this function?" | call sites with `symbol-id` + range. |
| | `definition` | "resolve this usage to its definition (go-to-def)" | the defining node, via `@local.scope` / `@local.definition` resolution. |
| **Verify** | `check` | "did my edit break the syntax?" | validity + locations of ERROR / MISSING nodes. Closes read → locate → edit → **verify**. |
| **Learn** | `ast` | "show node shapes so I can write a query" | S-expr / CST with ranges. Bootstraps better `query` calls. |

Why this exact set — it covers the whole loop, where 5 tools left five of six phases
with a hole:
- **`map`** answers the cold-start question `outline` (per-file) can't.
- **`source`** closes the "read the actual code" gap — without it the agent takes a
  range from `locate` and leaks the read back to the harness.
- **`callers` + `definition`** make tracing relationships first-class — the two most
  reached-for code-nav operations — instead of an afterthought.
- **`check`** finally plays tree-sitter's strongest card (error-tolerant parsing) and
  closes the editing loop our own *locate-don't-mutate* stance opens.
- **`grep`** is a deliberate, owned exception to the AST-purist framing: the agent
  shouldn't context-switch to a different tool mid-trace, and scope-aware text search
  is something only an AST engine can offer.

Four choices that make these good *agent* tools:
1. **Stable `symbol-id` handles** (e.g. `py:src/server.py#handle_request`) survive
   across turns, so the agent can `map`/`outline` now and `source`/`callers` that id
   three turns later without re-reading.
2. **Read-only, locate-don't-mutate** — `grove` returns ranges and code; the agent's
   Edit tool mutates. Trivial trust story, composes with any harness. (`check` is the
   read-only counterpart that lets the agent confirm its own edits.)
3. **Errors that teach** — malformed queries return the error plus nearby valid node
   kinds, so the agent fixes itself in one turn.
4. **Tiered + bounded by default** — `map`, `outline`, and `source` accept a detail
   level (0–3) and a token budget, so the high-value tools can't blow the
   token-frugal promise.

### 6.4 Verify & staleness — closing the loop with tree-sitter's best features

Our vision is built on *locate-don't-mutate*, but the agent then edits — which raises
two problems our first cut ignored, both solved by the two tree-sitter features we
were otherwise leaving on the table:

- **Verify (error-tolerant parsing).** After an edit the agent needs to know it didn't
  break syntax. tree-sitter parses through errors and marks **ERROR / MISSING** nodes;
  `check(path)` surfaces validity + their locations. This is *not* a language server —
  it reports syntactic breakage, not type or semantic diagnostics (still a non-goal).
- **Staleness (incremental re-parsing).** An edit makes the index stale; the next
  `symbols`/`outline` would lie. Every index-backed result carries a **staleness
  signal**, and `grove` re-parses incrementally — only the edited span, not the file —
  via a file-watcher (auto) with an explicit `refresh` fallback. Incremental reparse is
  tree-sitter's other headline feature; using it is what keeps the index honest at
  keystroke cost.

### 6.4.1 Field learning — availability ≠ adoption

In a Phase-1 test, grove's MCP server was registered (`.mcp.json` present, six tools
available) and a fresh agent session was asked a textbook code-navigation question.
**The agent did not use grove.** It fanned out a search agent and grepped instead.

Root cause, confirmed: (1) no instruction told the agent to prefer grove — the intent
lived in a human-facing test guide the agent never loads; (2) MCP tools are *deferred*
— presented as bare names, requiring a schema-load step before the first call — so an
undecided agent takes the zero-friction default (grep / read file). Tool descriptions
alone did not overcome a strong built-in habit.

The lesson reshapes the product, not just a doc:
- **`init` must write a steering directive**, not only a registration. The directive
  (in the agent's own instruction file) is what converts *available* into *used*, and
  what makes the agent load the deferred schemas up front. This is now a first-class
  `init` output and a per-agent responsibility of the config writers (§10).
- **Measure adoption, not just availability.** A "no-nudge" scenario that checks
  whether an agent reaches for grove unprompted is a real test — and its honest early
  verdict is "organic pull is insufficient; ship the directive." Test beds should keep
  a directive-free baseline *and* a with-directive run, so we see both numbers.

### 6.5 Grammar management — npm-like, lockfile-backed
```
$ grove add kotlin      # explicit add (usually unnecessary — init auto-adds)
$ grove list            # languages + versions in this project
$ grove update          # bump within lockfile constraints
```
`grove.lock` pins grammar **name + version + content hash**. Commit it; teammates,
CI, and the agent on any machine get byte-identical parsing. For a language with no
published grammar, `grove add ./my-grammar` accepts a local source, compiles to WASM
once, and caches the result — the long tail works without a per-machine toolchain.

### 6.6 One engine, two faces
The same binary is a **human CLI** and an **agent MCP server**:
```
$ grove outline src/server.py
  class  Server            12:0   src/server.py#Server
    def  __init__          14:4     (self, host, port)
    def  handle_request    31:4     (self, req) -> Response
  def    main              88:0   src/server.py#main
```
Symmetry means the developer can always reproduce and sanity-check what the agent
sees — debuggable, trustworthy.

### 6.7 Headless, multi-agent, CI
- `grove init --agent codex | claude | --all` for explicit targeting; auto-detect
  otherwise.
- `grove init --yes --quiet` for scripted/sandbox provisioning (agents bootstrapping
  themselves).
- `grove serve` runs the MCP server standalone for harnesses that prefer a long-lived
  process.
- In CI, `grove.lock` + the global cache means warm, deterministic, no-network runs.

---

## 7. Architecture — locked decisions

Two decisions, both confirmed, drive everything downstream.

### 7.1 One binary, two faces *(confirmed)*
A shared Rust core wrapping the `tree-sitter` and `tree-sitter-tags` crates,
exposed as both:
- `grove serve` — an MCP tool source, and
- `grove <verb>` — CLI verbs.

Same engine, same outputs — the human-debuggable symmetry comes for free. Maximal
agent compatibility; the developer can reproduce any agent result by hand.

### 7.2 Hosted WASM registry as the spine *(confirmed)*
Grammars are delivered as versioned WASM:
`name → versioned .wasm + bundled .scm queries`, content-hashed, cached in the
OS-native cache dir (`~/.cache/grove/grammars` etc.), pinned by `grove.lock`. WASM
is the **toolchain-free
guarantee** that lets `grove` run inside any agent sandbox. The registry is the
critical-path component — nothing else works until grammars resolve.

### 7.3 Implementation language
**Rust** is the default: links the tree-sitter crates in-process, ships a single
static binary, and runs WASM grammars via an embedded engine (`wasmtime`). *(To be
confirmed.)*

### 7.4 Repo posture
`grove` is a **separate greenfield project that depends on tree-sitter**, developed
outside the tree-sitter source tree. The upstream tree-sitter repo has a strict
no-unsupervised-AI contributing policy (`docs/src/6-contributing.md`), so nothing
here is committed there.

---

## 8. What `grove` reuses from tree-sitter (and what it doesn't)

`grove` is a thin shell over a slice of tree-sitter. Concrete mapping:

| Keep (the engine) | Drop (not agent-relevant) |
|---|---|
| `lib/` C runtime — parser + query engine | `crates/cli` — the full binary, playground, arg parsing |
| `lib/binding_rust` — `Parser`, `Query`, `QueryCursor`, `Node` | `crates/generate` — grammar → C compiler (agents consume, not author) |
| `crates/tags` — `TagsContext::generate_tags`, `Tag { name, kind, range, is_definition, docs }` | `crates/highlight` — terminal/HTML rendering |
| `crates/loader` *(optional)* — for the bring-your-own-grammar path | `init`, `test`, `fuzz`, `version`, docs, fixtures |

Tool → API mapping:
- `map` / `outline` / `symbols` / `source` → `crates/tags/src/tags.rs`
  (`TagsContext::generate_tags`, `@definition.*` / `@reference.*` / `@name` / `@doc`
  captures). `source` resolves a `symbol-id` to its `Tag.range` and slices the file;
  `map` aggregates per-file tags into a tiered tree.
- `callers` / `definition` → the same tags stream plus `@local.scope` /
  `@local.definition` / `@local.reference` captures for scope-aware resolution.
- `query` → `Query::new` + `QueryCursor::matches` (`lib/binding_rust/lib.rs`),
  S-expression patterns, predicates, byte-range filtering via `set_byte_range`.
- `grep` → regex over file bytes, with the parse tree used to classify each match's
  node kind (code vs comment vs string) for scope-aware mode.
- `locate` → smallest-named-node-at-position + ancestor walk over `Node`.
- `check` → parse + walk for `Node::is_error()` / MISSING nodes; report their ranges.
- `ast` → the parse output formats already modeled in `crates/cli/src/parse.rs`
  (S-expr / CST with ranges).
- **Staleness / refresh** → `Tree::edit` + `Parser::parse(text, Some(old_tree))` for
  incremental re-parse of only the edited span; `notify`-style watcher drives auto
  refresh, with an explicit `refresh` verb as fallback.
- Cancellation → the tags and highlight APIs already take a cancellation flag; wire
  it to the harness interrupt so long repo scans are killable.

---

## 9. Build order / roadmap

> **Status (built so far):** Phases 0–1 done (six tools, both faces, validated on a
> ripgrep test bed). Phase 2 step 1–3 done: grammars now load **at runtime from a
> WASM registry** (local-directory stub), proven multi-language (rust + python with
> zero recompile), tag extraction reimplemented over the Query engine (since
> `tree-sitter-tags` can't drive wasm languages), `grove.lock` with wasm sha256, and
> `grove languages`/`grove lock`. **`grove init` is built** — detects languages and
> writes `.mcp.json` (availability) + `CLAUDE.md` steering directive (adoption) +
> `grove.lock`, idempotently and non-destructively (the Claude Code config writer).
> `grove fetch` downloads grammars from a hosted registry into the OS-native cache,
> verifying each wasm's sha256 against the catalog (host model: a `grove-registry`
> GitHub repo via jsDelivr's CDN, overridable with `GROVE_REGISTRY_URL`).
> Remaining: publish the actual hosted registry repo, artifact signing, more
> agent-config writers (Codex/Cursor/…), and the `map`/`grep`/`ast` tools.

The registry is the spine — sequence the work so nothing blocks on hosting:

1. **Registry schema + resolver (the spine).** Manifest format, version + hash
   semantics, cache layout, `grove.lock`. Ship with a **local-directory registry
   stub** so the engine work isn't blocked on hosting infrastructure.
2. **Core engine + tool surface.** Build by loop phase, simplest first: Read/Find
   (`outline`, `symbols`, `query`, `grep`, `source`) → Locate (`locate`) → Trace
   (`callers`, `definition`) → Verify (`check`) → Orient (`map`) → Learn (`ast`),
   against the resolved-grammar interface, including the stable `symbol-id` scheme.
   Add the staleness signal + incremental `refresh` once the index-backed tools exist.
3. **Two faces.** Wrap the engine as `grove serve` (MCP) and `grove <verb>` (CLI).
4. **`grove init`.** Detect languages → resolve grammars → write lockfile + agent
   registration **+ a steering directive** (§6.4.1). The per-agent config adapters
   come last and grow over time.

A throwaway **spike** can precede step 1: shell out to the existing `tree-sitter
tags/query/parse` CLI to prove the agent loop benefits before building the real
engine.

---

## 10. Missing components (the greenfield gaps)

This UX implies building things that don't exist yet:

1. **Grammar registry + WASM distribution** — a hosted index
   (`name → versioned .wasm + bundled .scm queries`) and a resolver. The single
   biggest net-new piece; grammars today are scattered repos compiled per machine.
2. **Lockfile + content-hash format** (`grove.lock`) and the global cache layout.
3. **Agent-config writers** — one small adapter per harness (Claude Code, Codex,
   antigravity, pi, …) that writes *both* the tool registration (MCP/CLI) *and* the
   steering directive into that agent's instruction file (`CLAUDE.md` / `AGENTS.md` /
   `.cursorrules`). Availability without steering does not get the tools used (§6.4.1).
4. **Stable symbol-id scheme** — a deterministic addressing convention over
   tree-sitter's tags output that survives edits well enough to be referenced across
   turns.
5. **LLM-tuned tool descriptions + teaching errors** — the prompt-engineering layer
   that makes the agent *use the tools well*. Product work, not parser work.

Tree-sitter supplies the entire engine for the raw data behind every tool; the new
build is the distribution, registration, and agent-ergonomics shell around it.

---

## 11. Design principles (the throughline)

- **One command does everything; everything else is the agent.** The developer's
  mental budget is a single `grove init`.
- **Autodetect over configure.** Languages, agent type, grammar versions — inferred,
  overridable, never required.
- **Token-frugal by default.** Bounded structured outputs; verbosity is opt-in.
- **Stable addresses across the loop.** The thing that turns isolated tool calls
  into a coherent read → locate → edit cycle.
- **No toolchain on the consumer.** WASM grammars, because agents live in sandboxes.
- **Locate, don't mutate.** Precise locations out; the agent's Edit tool does the
  writing.

---

## 12. Open questions

- Confirm **Rust** as the implementation language (vs. a Go/TS shell over the C lib).
- Registry hosting + governance: who publishes grammars, how versions are vetted,
  trust/signing of WASM artifacts.
- `symbol-id` stability strategy across edits — content-relative vs. positional.
- Which agents get first-class registration adapters at launch.
- Default token budgets per tool and how the agent requests more.
