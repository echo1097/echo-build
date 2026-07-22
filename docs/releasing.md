# Echo Build source releases

GitHub repository `echo1097/echo-build` is the only distribution origin. Releases are
immutable SemVer tags. Stable releases use `vMAJOR.MINOR.PATCH`; beta releases use a
prerelease such as `v0.3.0-beta.1`. There is no development update channel.

## Trust and ownership

Only repository maintainers with tag permission may create a release tag. Protect
`v*` tags in GitHub, require reviewed release changes, and require the `CI` workflow
to pass before tagging. Pull-request jobs receive no release or signing credentials.

The first source-release series does not require signed tags. This is an explicit
tradeoff: repository permissions and HTTPS protect publication while avoiding an
unowned signing-key dependency. If signing is enabled later, the installer must pin
the allowed fingerprint before enforcement is switched on. If that key is
compromised, revoke its GitHub access, publish the fingerprint revocation through a
reviewed repository change, disable releases, rotate the pinned fingerprint, audit
tags, and publish a new version. Never move an existing tag.

## Release procedure

1. Update `crates/codegen/xai-grok-pager-bin/Cargo.toml` and `Cargo.lock`. The tag
   must equal `v` plus that package version.
2. Update the Echo release notes from the current diff and have the release change
   reviewed.
3. Run CI, including the clean macOS and Linux `release-dist` builds, PTY smoke test,
   credential-store smoke tests where services exist, dependency/license/secret
   scans, and installer tests.
4. From a clean checkout, run the release check and build with `--locked`.
5. Create the immutable tag, then test a latest install, upgrade, explicit rollback,
   interrupted build, and a forced startup failure before announcing it.
6. Retain the Git revision, redacted CI logs, and release notes with the GitHub run.

If a release is bad, mark its GitHub release as deprecated and name the replacement
in its notes. Publish a higher version containing the fix. Never delete, move, or
reuse the bad tag.

## User operations

Install or upgrade the latest stable release:

```sh
curl --proto '=https' --tlsv1.2 --fail \
  https://raw.githubusercontent.com/echo1097/echo-build/main/install.sh | sh
```

Install or roll back to an exact version:

```sh
sh install.sh --version v0.2.106
sh install.sh --version v0.2.105 --allow-downgrade
```

Reinstall by rerunning the installer with the desired version. Roll back to the
last working executable without rebuilding with
`mv ~/.local/bin/echo-build.previous ~/.local/bin/echo-build`. Uninstall with:

```sh
rm ~/.local/bin/echo-build ~/.local/bin/echo-build.previous
rm -rf "${XDG_DATA_HOME:-$HOME/.local/share}/echo-build/source"
```

If replacing the running executable is restricted on a platform, exit all Echo
Build processes and rerun the installer. The staged and previous executables make
the operation recoverable.
