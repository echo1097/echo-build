# Echo Build 0.2.106

## OpenRouter release

- Secure OpenRouter API keys with credential-store login and `/auth` management.
- OpenRouter-only inference with protected credential routing.
- Rich model details with capabilities and accurate context windows.

## Improvements

- Added the Echo-owned macOS and Linux source installer, exact SemVer tag builds, explicit `echo-build update` checks and installs, atomic replacement, previous-binary preservation, and tested rollback recovery.
- Added clean macOS and Linux release CI, tag-to-binary version enforcement, installer simulations, credential-store and PTY smoke jobs, dependency and license checks, and committed-secret scanning.
- Established the public Echo Build contract: the executable and Cargo artifact are now `echo-build`, the default state root is `~/.echo-build`, and canonical environment variables use the `ECHO_BUILD_` prefix.
- Added canonical `echo.build/*` ACP extensions and `echo.openrouter/*` authentication methods with discoverable capability metadata. Temporary `x.ai/*` and selected `GROK_*` aliases are scheduled for removal in 0.3.0.
- Updated process restart, leader discovery, shell completions, terminal titles, notifications, and source-build documentation to use the Echo Build identity.
- Added secure OpenRouter API key login backed only by the operating system credential store, plus `/auth` for updating or clearing the key.
- Routed inference and model discovery through OpenRouter, with OpenRouter Auto as the default and safeguards that prevent credentials from being sent to legacy provider hosts.
- Expanded the model picker with provider slugs, agent capability labels, image support, reasoning support, and accurate per-model context windows.
- Improved OpenRouter streaming with provider error handling, partial-response failure reporting, request cost tracking, and clearer context-limit messages.
- Rebranded the welcome experience for the SpaceXAI OpenRouter fork and removed Grok subscription, billing, and voice promotion paths from the TUI.
- Simplified plan mode and session controls, including clearer active-model details and context usage after switching models.

## Security

- Removed production API key reads from environment variables, plaintext configuration, session credentials, logs, telemetry, and attribution callbacks.
- Restricted authenticated sampling requests to OpenRouter and loopback development servers.
