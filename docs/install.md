# Install

grove ships as a single static binary (Linux, macOS, Windows; x86_64 + aarch64
where applicable). No grammar is compiled in — they load at runtime from the
[hosted registry](languages.md).

## curl | sh (Linux / macOS)

Detects your platform, verifies the sha256, installs to `~/.local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/Entelligentsia/grove/main/install.sh | sh
```

Honors `HTTP_PROXY`/`HTTPS_PROXY`/`ALL_PROXY` + `NO_PROXY` via `curl`/`wget`.

## PowerShell (Windows)

Verifies the sha256, installs to `$env:LOCALAPPDATA\grove\bin`:

```powershell
irm https://raw.githubusercontent.com/Entelligentsia/grove/main/install.ps1 | iex
```

Honors `HTTPS_PROXY`/`HTTP_PROXY`/`ALL_PROXY` + `NO_PROXY` from the environment
(set them before running the one-liner). For an explicit `-Proxy`, download the
script first — piping into `iex` doesn't accept parameters:

```powershell
iwr https://raw.githubusercontent.com/Entelligentsia/grove/main/install.ps1 -OutFile install.ps1
./install.ps1 -Proxy "http://proxy.corp:8080"
```

`-InstallDir` and `-Version` (or `$env:GROVE_INSTALL_DIR` / `$env:GROVE_VERSION`)
override the defaults, mirroring `install.sh`'s env vars. Only the
`x86_64-pc-windows-msvc` prebuilt is published today; other Windows archs fall
back to `cargo install`.

## Homebrew (macOS / Linux)

```bash
brew install Entelligentsia/grove/grove
```

Tap: [`Entelligentsia/homebrew-grove`](https://github.com/Entelligentsia/homebrew-grove).

## npm

Provides the `grove` binary via a downloaded, checksum-verified prebuilt:

```bash
npm install -g @entelligentsia/grove
```

## From source

Install the published crate (installs the `grove` binary):

```bash
cargo install grove-cst-cli
```

Or straight from git — the repo is a workspace, so name the CLI package:

```bash
cargo install --git https://github.com/Entelligentsia/grove grove-cst-cli
```

Build from a checkout:

```bash
cargo build --release      # first build compiles wasmtime (~30s), then incremental
# binary at target/release/grove
```

## As an agent skill (cross-harness)

The skill works across 70+ harnesses (Claude Code, Cursor, Codex, Cline, …) via
the [agent-skills tool](https://github.com/vercel-labs/skills):

```bash
npx skills add Entelligentsia/grove
```

The skill steers your agent to grove's MCP tools when present, else the `grove`
CLI — and **self-installs the binary on first use** if it's missing (`npm i -g
@entelligentsia/grove`, then `grove init --as skill` to fetch grammars). So this
one command is enough to get grove working in a fresh repo. See
[Setup](setup.md) for `grove init --as mcp|skill|both`.

## Prebuilt binaries

Attached to each [GitHub Release](https://github.com/Entelligentsia/grove/releases).

---

Next: [Setup](setup.md) · [Languages & grammars](languages.md)
