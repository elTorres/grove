# Distribution

How grove ships. The single source of truth is the **GitHub Release** for a tag;
every channel pulls prebuilt binaries from it.

## Targets

| Target                          | Runner            | Asset                                      |
| ------------------------------- | ----------------- | ------------------------------------------ |
| `x86_64-unknown-linux-gnu`      | `ubuntu-latest`   | `grove-x86_64-unknown-linux-gnu.tar.gz`    |
| `aarch64-unknown-linux-gnu`     | `ubuntu-24.04-arm`| `grove-aarch64-unknown-linux-gnu.tar.gz`   |
| `x86_64-apple-darwin`           | `macos-13`        | `grove-x86_64-apple-darwin.tar.gz`         |
| `aarch64-apple-darwin`          | `macos-14`        | `grove-aarch64-apple-darwin.tar.gz`        |
| `x86_64-pc-windows-msvc`        | `windows-latest`  | `grove-x86_64-pc-windows-msvc.zip`         |

Each asset has a `.sha256` sidecar. The installer, npm wrapper, and Homebrew
formula all verify against it.

## Cutting a release

1. Bump `version` in `Cargo.toml` (and `dist/npm/package.json` to match), commit.
2. Tag and push: `git tag v0.1.0 && git push origin v0.1.0`.
3. `.github/workflows/release.yml` fires on the tag: it builds all five targets
   on native runners, packages + checksums each, and creates the GitHub Release
   with the assets attached.
4. **Homebrew:** `dist/homebrew/update-formula.sh v0.1.0 > grove.rb`, then copy
   the result into the tap repo `Entelligentsia/homebrew-grove` as
   `Formula/grove.rb`. (The script fetches the published `.sha256` sidecars.)
5. **npm:** `cd dist/npm && npm publish --access public` (scope:
   `@entelligentsia`). Its `postinstall` downloads the matching prebuilt.

## Channels

- **`install.sh`** (`curl | sh`) — platform detection, checksum-verified, installs
  to `$GROVE_INSTALL_DIR` (default `~/.local/bin`). `GROVE_VERSION` pins a tag.
- **Homebrew** — `dist/homebrew/grove.rb` template; published via the tap.
- **npm** — `dist/npm/` thin wrapper; `bin/grove.js` execs the vendored prebuilt
  that `install.js` downloads at install time.
- **`cargo install --git`** — builds from source. `grove` is taken on crates.io,
  so there is no published crate; install straight from the repo.

## CI

`.github/workflows/ci.yml` builds + tests (`--locked`) on Linux for every push to
`main`/`master` and every PR. Lint (`clippy`/`fmt`) and a build matrix are
intentional follow-ups.
