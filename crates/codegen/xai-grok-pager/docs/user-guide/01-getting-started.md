# Getting Started

Echo Build is a SpaceXAI-branded terminal coding agent for the OpenRouter
ecosystem. It runs interactively as a full-screen TUI, headlessly for scripts,
or through the Agent Client Protocol (ACP).

## Build from source

No Echo-owned installer or release channel is configured yet. Build the public
binary from this repository:

```sh
cargo build -p xai-grok-pager-bin --release --bin echo-build
./target/release/echo-build --version
```

The `xai-*` package names are retained internal implementation details. The
public executable is `echo-build`; no `grok` compatibility executable ships.

## First launch

```sh
echo-build
```

Enter an OpenRouter API key when prompted. Echo Build stores it only in the
operating-system credential store. The login screen does not start browser,
device-code, Grok account, or subscription authentication.

See [Authentication](02-authentication.md) for the credential and network
security boundaries.

## State and configuration

User state defaults to `~/.echo-build`. Override it with:

```sh
export ECHO_BUILD_HOME="$HOME/custom-echo-home"
```

`GROK_HOME` is a deprecated compatibility read through 0.2.x and is scheduled
for removal in 0.3.0. Project-local inherited `.grok` paths and legacy state
migration are handled separately from the public product identity.

## Basic interaction

Type a prompt and press `Enter`. Echo Build can inspect and edit files, run
commands, search, and track longer tasks. It asks for permission before risky
tool execution unless you explicitly enable always-approve mode.

- Press `Ctrl+O` to toggle always-approve mode.
- Press `Ctrl+C` during a turn to cancel it.
- Use `@path/to/file` to attach a file.
- Type `/` to browse slash commands.
- Use `/auth` to update or clear the OpenRouter key.

## Common commands

```sh
# Start in a project
echo-build --cwd ~/projects/my-app

# Start with an initial prompt
echo-build "fix the failing test"

# Run one prompt non-interactively
echo-build -p "explain this codebase"

# Resume a session
echo-build --resume <session-id>

# Continue the most recent session
echo-build -c

# Use a worktree
echo-build --worktree=feat "implement the feature"

# Generate shell completions
echo-build completions zsh
```

Sessions are stored beneath `~/.echo-build/sessions` unless
`ECHO_BUILD_HOME` overrides the root.

## ACP extensions

Clients should use `echo.build/*` for runtime extensions and
`echo.openrouter/*` for authentication. Initialize metadata and
`echo.build/capabilities` advertise these namespaces. Legacy `x.ai/*` requests
remain compatibility aliases through 0.2.x and are scheduled for removal in
0.3.0.

## Project instructions

Add an `AGENTS.md` file to a repository to provide project-specific rules.
Echo Build reads the closest applicable instructions when starting work.
