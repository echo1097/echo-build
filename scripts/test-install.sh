#!/bin/sh

set -eu

projectRoot=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
testRoot=$(mktemp -d)
trap 'rm -rf "$testRoot"' EXIT INT TERM

fixture="$testRoot/fixture"
mkdir -p "$fixture/crates/codegen/xai-grok-pager-bin" "$testRoot/bin" "$testRoot/home"
cp "$projectRoot/install.sh" "$fixture/install.sh"

writeVersion() {
    version=$1
    printf '[package]\nname = "xai-grok-pager-bin"\nversion = "%s"\n' "$version" > "$fixture/crates/codegen/xai-grok-pager-bin/Cargo.toml"
    git -C "$fixture" add .
    git -C "$fixture" commit -q -m "version $version"
    git -C "$fixture" tag "v$version"
}

git -C "$fixture" init -q
git -C "$fixture" config user.name "Echo Build tests"
git -C "$fixture" config user.email "tests@example.invalid"
writeVersion 1.0.0
writeVersion 1.1.0

realGit=$(command -v git)
ln -s "$realGit" "$testRoot/bin/git"

cat > "$testRoot/bin/cargo" <<'EOF'
#!/bin/sh
set -eu
version=$(awk -F '"' '/^version =/ { print $2; exit }' crates/codegen/xai-grok-pager-bin/Cargo.toml)
revision=$(git rev-parse --short HEAD)
mkdir -p target/release-dist
cat > target/release-dist/echo-build <<BINARY
#!/bin/sh
if [ "$version" = "1.2.0" ] && ! printf '%s' "\$0" | grep -q '/target/release-dist/'; then
    exit 86
fi
if [ "\${1:-}" = "--version" ]; then
    printf '%s\n' "echo-build $version ($revision)"
    exit 0
fi
exit 0
BINARY
chmod +x target/release-dist/echo-build
EOF
chmod +x "$testRoot/bin/cargo"
cat > "$testRoot/bin/dotslash" <<'EOF'
#!/bin/sh
exit 0
EOF
chmod +x "$testRoot/bin/dotslash"
ln -s /usr/bin/cc "$testRoot/bin/cc" 2>/dev/null || ln -s "$(command -v cc)" "$testRoot/bin/cc"

runInstaller() {
    env \
        HOME="$testRoot/home" \
        PATH="$testRoot/bin:/usr/bin:/bin" \
        ECHO_BUILD_INSTALLER_TESTING=1 \
        ECHO_BUILD_TEST_REPOSITORY_URL="$fixture" \
        ECHO_BUILD_INSTALL_DIR="$testRoot/home/bin" \
        XDG_DATA_HOME="$testRoot/home/data" \
        sh "$projectRoot/install.sh" "$@"
}

if runInstaller --version main 2>/dev/null; then
    printf '%s\n' "installer accepted a branch" >&2
    exit 1
fi

if runInstaller --version v9.9.9 2>/dev/null; then
    printf '%s\n' "installer accepted a missing tag" >&2
    exit 1
fi

runInstaller --version v1.0.0
test "$("$testRoot/home/bin/echo-build" --version | awk '{print $2}')" = "1.0.0"

runInstaller
test "$("$testRoot/home/bin/echo-build" --version | awk '{print $2}')" = "1.1.0"
test -x "$testRoot/home/bin/echo-build.previous"

if runInstaller --version v1.0.0 </dev/null 2>/dev/null; then
    printf '%s\n' "installer allowed a non-interactive downgrade without a flag" >&2
    exit 1
fi

runInstaller --version v1.0.0 --allow-downgrade
test "$("$testRoot/home/bin/echo-build" --version | awk '{print $2}')" = "1.0.0"

writeVersion 1.2.0
printf '%s\n' fail-after-copy > "$fixture/fail-after-copy"
git -C "$fixture" add fail-after-copy
git -C "$fixture" commit -q -m "simulate failed startup"
git -C "$fixture" tag -f v1.2.0

if runInstaller --version v1.2.0 2>/dev/null; then
    printf '%s\n' "installer accepted a failed startup check" >&2
    exit 1
fi
test "$("$testRoot/home/bin/echo-build" --version | awk '{print $2}')" = "1.0.0"

test ! -e "$testRoot/home/bin/grok"
printf '%s\n' "installer install, upgrade, preservation, failed-startup recovery, and rollback tests passed"
