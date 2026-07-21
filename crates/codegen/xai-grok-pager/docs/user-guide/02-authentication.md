# Authentication

Echo Build uses an OpenRouter API key. The interactive TUI does not start a
browser, device-code flow, Grok login, or SpaceXAI subscription flow.

## Sign in

Run the public executable:

```sh
echo-build
```

On first launch, enter an OpenRouter API key in the TUI. Echo Build stores the
key only in the operating-system credential store under the `echo-build`
service. It never writes the key to `auth.json`, `config.toml`, session files,
logs, telemetry, snapshots, or error messages.

If the credential store is unavailable, authentication fails securely. There
is no plaintext or session-only persistence fallback.

## Update or remove the key

Use `/auth` in the TUI to replace or clear the stored key. The CLI logout path
is also available:

```sh
echo-build logout
```

Clearing the key removes it from the operating-system credential store and
from resident process memory.

## State directory

User state defaults to:

```text
~/.echo-build
```

Override it with `ECHO_BUILD_HOME`. `GROK_HOME` remains a deprecated read-time
compatibility alias through the 0.2 release line and is scheduled for removal
in 0.3.0. API keys are not stored in either directory.

Legacy non-secret state migration is covered separately by the state and
configuration migration phase. Echo Build does not import plaintext bearer
tokens or API keys from legacy files.

## Network boundary

Production credentials are sent only to `https://openrouter.ai`. Plain HTTP is
accepted only for an explicitly configured loopback development endpoint.
