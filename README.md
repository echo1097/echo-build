# Echo Build (`echo-build`)

**Echo Build** is a fork of Grok Build by SpaceXAI for the OpenRouter ecosystem. It is a
full-screen TUI that understands your codebase, edits files, executes shell
commands, searches the web, and manages long-running tasks — interactively,
headlessly for scripting/CI, or embedded in editors via the Agent Client
Protocol (ACP).

[Install](#install) ·
[Building from source](#building-from-source) ·
[Authentication](#authentication) ·
[Documentation](#documentation) ·
[Repository layout](#repository-layout) ·
[Development](#development) ·
[Contributing](#contributing) ·
[License](#license)

This repository contains the Rust source for the `echo-build` CLI/TUI and its
agent runtime. Most internal `xai-*` crate and module names are retained as
upstream implementation details to keep the fork boundary narrow.

A small `SOURCE_REV` file at the root records the full monorepo commit SHA
for the version of the code present in this tree.

Echo Build is distributed only as versioned source tags from the Echo-owned
[`echo1097/echo-build`](https://github.com/echo1097/echo-build) repository. The
installer builds locally; there are no prebuilt binaries, npm packages, automatic
update checks, or background updates.

## Install

The installer supports macOS and Linux on x86-64 and ARM64. It requires Git,
Rust/Cargo, a C linker, and platform build tools. On macOS install Xcode Command
Line Tools; on Debian or Ubuntu install `build-essential`. The build also uses
DotSlash and `protoc` as described below.

```sh
curl --proto '=https' --tlsv1.2 --fail \
  https://raw.githubusercontent.com/echo1097/echo-build/main/install.sh | sh
```

This selects the latest stable SemVer tag, checks it out in detached HEAD state,
builds with the locked distribution profile, and atomically installs
`~/.local/bin/echo-build`. It never creates a `grok` executable. Installation and
updates are explicit:

```sh
echo-build update --check
echo-build update
echo-build update --version v0.2.106
echo-build update --version v0.2.105 --allow-downgrade
```

See [the source release guide](docs/releasing.md) for reinstall, rollback,
uninstall, release trust, and bad-release recovery procedures.

## Building from source

Requirements:

- **Rust** — the toolchain is pinned by [`rust-toolchain.toml`](rust-toolchain.toml);
  `rustup` installs it automatically on first build.
- **[DotSlash](https://dotslash-cli.com)** — required so hermetic tools under
  [`bin/`](bin/) (notably [`bin/protoc`](bin/protoc)) can download and run.
  Install it and ensure `dotslash` is on your `PATH` **before** building:

  ```sh
  cargo install dotslash
  # or: prebuilt packages — https://dotslash-cli.com/docs/installation/
  /usr/bin/env dotslash --help   # sanity check
  ```

- **protoc** — proto codegen resolves [`bin/protoc`](bin/protoc) via DotSlash,
  or falls back to a `protoc` on `PATH` / `$PROTOC`.
- macOS and Linux are supported build hosts; Windows builds are best-effort
  and not currently tested from this tree.

```sh
cargo run -p xai-grok-pager-bin --bin echo-build
cargo build --locked --profile release-dist -p xai-grok-pager-bin --bin echo-build
cargo check -p xai-grok-pager-bin
```

The distribution binary artifact is `target/release-dist/echo-build`.

User state defaults to `~/.echo-build`. Set `ECHO_BUILD_HOME` to override it.
`GROK_HOME` and selected `GROK_*` environment aliases are compatibility-only
through the 0.2 release line and are scheduled for removal in 0.3.0.

## Authentication

On first launch, the TUI asks for an OpenRouter API key. Echo Build stores the
key only in the operating-system credential store and keeps a non-persistent
in-memory copy while the process is running.

- Browser login, device-code login, OIDC, Grok subscriptions, and cached Grok
  sessions are not supported authentication methods.
- Environment variables and per-model `api_key` / `env_key` settings are not
  OpenRouter credential sources. Catalog responses also cannot override the
  API key, request headers, or OpenRouter base URL.
- Saving a replacement key updates the running process only after secure
  persistence succeeds. Empty keys are rejected without changing the current
  credential.
- `logout` and `clear key` perform the same idempotent operation: they remove
  the persistent key and its in-memory copy.
- If OpenRouter rejects a request with HTTP 401, the TUI returns to the same
  supported OpenRouter key-entry flow. It never falls back to browser login.
- ACP authentication status and info responses expose only whether a key is
  configured; the key itself is never returned.

If the operating-system credential store is unavailable, Echo Build fails
closed and sampling remains blocked. See the
[authentication guide](crates/codegen/xai-grok-pager/docs/user-guide/02-authentication.md)
for the interactive, headless, and ACP flows.

## Documentation

The user guide ships with the pager crate:
[`crates/codegen/xai-grok-pager/docs/user-guide/`](crates/codegen/xai-grok-pager/docs/user-guide/)
— getting started, keyboard shortcuts, slash commands, configuration, theming,
MCP servers, skills, plugins, hooks, headless mode, sandboxing, and more.

## Repository layout

| Path | Contents |
|------|----------|
| `crates/codegen/xai-grok-pager-bin` | Composition-root package; builds the public `echo-build` binary |
| `crates/codegen/xai-grok-pager` | The TUI: scrollback, prompt, modals, rendering |
| `crates/codegen/xai-grok-shell` | Agent runtime + leader/stdio/headless entry points |
| `crates/codegen/xai-grok-tools` | Tool implementations (terminal, file edit, search, ...) |
| `crates/codegen/xai-grok-workspace` | Host filesystem, VCS, execution, checkpoints |
| `crates/codegen/...` | The rest of the CLI crate closure (config, MCP, markdown, sandbox, ...) |
| `crates/common/`, `crates/build/`, `prod/mc/` | Small shared leaf crates pulled in by the closure |
| `third_party/` | Vendored upstream source (Mermaid diagram stack) — see below |

> [!IMPORTANT]
> The root `Cargo.toml` (workspace members, dependency versions, lints,
> profiles) is **generated** — treat it as read-only. Prefer editing per-crate
> `Cargo.toml` files.

## Development

```sh
cargo check -p <crate>        # always target specific crates; full-workspace builds are slow
cargo test -p xai-grok-config # per-crate tests
cargo clippy -p <crate>       # lint config: clippy.toml at the repo root
cargo fmt --all               # rustfmt.toml at the repo root
```

## Contributing

> [!NOTE]
> External contributions are not accepted at the moment

## License

First-party code in this repository is licensed under the **Apache License,
Version 2.0** — see [`LICENSE`](LICENSE).

Third-party and vendored code remains under its original licenses. See:

- [`THIRD-PARTY-NOTICES`](THIRD-PARTY-NOTICES) — crates.io / git dependencies,
  bundled UI themes, and **in-tree source ports** (including openai/codex and
  sst/opencode tool implementations)
- [`crates/codegen/xai-grok-tools/THIRD_PARTY_NOTICES.md`](crates/codegen/xai-grok-tools/THIRD_PARTY_NOTICES.md)
  — crate-local notice for the codex and opencode ports (license texts +
  Apache §4(b) change notice)
- [`third_party/NOTICE`](third_party/NOTICE) — vendored Mermaid-stack index
