# Echo Build agent rules

- The TUI login collects an OpenRouter API key. Do not start Grok browser, device-code, or token verification from this screen.
- Store API keys only in the operating system credential store. Never write them to `auth.json`, configuration files, logs, telemetry, snapshots, or error messages.
- Fail securely when the credential store is unavailable. Do not add plaintext or session-only persistence fallbacks.
- Keep OpenRouter endpoint, model, header, and routing changes out of scope until they are requested separately.
- Keep the welcome splash branded as a SpaceXAI fork for the OpenRouter ecosystem. Do not surface Grok subscription upgrade promotions, links, or actions in the TUI.
- Preserve unrelated worktree changes and run the relevant focused tests plus the pager binary build after frontend changes.
