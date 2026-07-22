#!/bin/sh

set -eu

repositoryUrl="https://github.com/echo1097/echo-build.git"
repositorySlug="echo1097/echo-build"
versionTag=""
allowDowngrade=0
checkOnly=0

usage() {
    printf '%s\n' "Usage: install.sh [--version vMAJOR.MINOR.PATCH] [--allow-downgrade] [--check]"
    printf '%s\n' ""
    printf '%s\n' "Build and install an exact Echo Build release tag from GitHub."
}

fail() {
    printf 'echo-build installer: %s\n' "$1" >&2
    exit 1
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --version)
            [ "$#" -ge 2 ] || fail "--version requires a tag"
            versionTag=$2
            shift 2
            ;;
        --allow-downgrade)
            allowDowngrade=1
            shift
            ;;
        --check)
            checkOnly=1
            shift
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            fail "unknown argument: $1"
            ;;
    esac
done

case "$(uname -s 2>/dev/null || true)" in
    Darwin|Linux) ;;
    *) fail "unsupported operating system; native Windows installation is not available" ;;
esac

case "$(uname -m 2>/dev/null || true)" in
    x86_64|amd64|arm64|aarch64) ;;
    *) fail "unsupported architecture: $(uname -m 2>/dev/null || printf unknown)" ;;
esac

command -v git >/dev/null 2>&1 || fail "Git is required; install Git and rerun this command"

if [ "${ECHO_BUILD_INSTALLER_TESTING:-0}" = "1" ]; then
    repositoryUrl=${ECHO_BUILD_TEST_REPOSITORY_URL:-$repositoryUrl}
fi

isVersionTag() {
    printf '%s\n' "$1" | grep -Eq '^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z]+([.-][0-9A-Za-z]+)*)?$'
}

latestStableTag() {
    git -c http.followRedirects=false ls-remote --tags --refs "$repositoryUrl" 'refs/tags/v*' |
        awk '
            {
                sub("refs/tags/", "", $2)
                tag=$2
            }
            tag ~ /^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$/ {
                split(substr(tag, 2), parts, ".")
                if (!found || parts[1] + 0 > major ||
                    (parts[1] + 0 == major && parts[2] + 0 > minor) ||
                    (parts[1] + 0 == major && parts[2] + 0 == minor && parts[3] + 0 > patch)) {
                    found=1
                    major=parts[1] + 0
                    minor=parts[2] + 0
                    patch=parts[3] + 0
                    latest=tag
                }
            }
            END { if (found) print latest }
        '
}

if [ -z "$versionTag" ]; then
    printf '%s\n' "Querying $repositoryUrl for the latest stable release tag..."
    versionTag=$(latestStableTag)
    [ -n "$versionTag" ] || fail "no stable version tags were found"
fi

isVersionTag "$versionTag" || fail "invalid release tag '$versionTag'; expected vMAJOR.MINOR.PATCH or a SemVer prerelease"

tagRevision=$(git -c http.followRedirects=false ls-remote --tags "$repositoryUrl" "refs/tags/$versionTag" "refs/tags/$versionTag^{}" |
    awk '/\^\{\}$/ { peeled=$1 } !/\^\{\}$/ { directRevision=$1 } END { if (peeled != "") print peeled; else print directRevision }')
[ -n "$tagRevision" ] || fail "release tag '$versionTag' does not exist in $repositorySlug"

installDir=${ECHO_BUILD_INSTALL_DIR:-"$HOME/.local/bin"}
installPath="$installDir/echo-build"
currentVersion="not installed"

if [ -x "$installPath" ]; then
    currentVersion=$($installPath --version 2>/dev/null | awk '{print $2}' | sed 's/[[:space:]].*$//' || true)
fi

if [ "$checkOnly" -eq 1 ]; then
    printf 'Installed: %s\nLatest stable: %s\n' "$currentVersion" "$versionTag"
    exit 0
fi

command -v cargo >/dev/null 2>&1 || fail "Rust and Cargo are required; install rustup from https://rustup.rs and rerun this command"
command -v cc >/dev/null 2>&1 || command -v clang >/dev/null 2>&1 || command -v gcc >/dev/null 2>&1 || fail "a C linker and platform build tools are required (Xcode Command Line Tools on macOS, build-essential on Debian/Ubuntu)"
command -v dotslash >/dev/null 2>&1 || fail "DotSlash is required for the pinned build tools; install it with 'cargo install dotslash' and rerun this command"

dataHome=${XDG_DATA_HOME:-"$HOME/.local/share"}
sourceDir="$dataHome/echo-build/source"
mkdir -p "$(dirname "$sourceDir")" "$installDir"

if [ -d "$sourceDir/.git" ]; then
    originUrl=$(git -C "$sourceDir" remote get-url origin)
    [ "$originUrl" = "$repositoryUrl" ] || fail "managed checkout has unexpected origin '$originUrl'; expected $repositoryUrl"
    printf '%s\n' "Fetching release tags from $repositoryUrl..."
    git -c http.followRedirects=false -C "$sourceDir" fetch --force --prune --tags origin
else
    [ ! -e "$sourceDir" ] || fail "$sourceDir exists but is not an Echo Build Git checkout"
    printf '%s\n' "Cloning Echo Build source from $repositoryUrl..."
    git -c http.followRedirects=false clone --filter=blob:none --no-checkout "$repositoryUrl" "$sourceDir"
    git -C "$sourceDir" remote set-url --push origin DISABLED
    git -c http.followRedirects=false -C "$sourceDir" fetch --force --tags origin
fi

git -C "$sourceDir" checkout --detach --force "$versionTag"
checkedRevision=$(git -C "$sourceDir" rev-parse HEAD)
[ "$checkedRevision" = "$tagRevision" ] || fail "checked-out revision does not match the selected tag"

packageVersion=$(awk '
    /^\[package\]$/ { inPackage=1; next }
    /^\[/ { inPackage=0 }
    inPackage && /^version[[:space:]]*=/ { gsub(/[[:space:]\"]/, "", $0); sub(/^version=/, "", $0); print; exit }
' "$sourceDir/crates/codegen/xai-grok-pager-bin/Cargo.toml")
[ "v$packageVersion" = "$versionTag" ] || fail "tag $versionTag does not match pager binary version $packageVersion"

if [ -x "$installPath" ] && [ "$allowDowngrade" -ne 1 ]; then
    currentTag="v$currentVersion"
    highestTag=$(git -C "$sourceDir" tag --sort=-version:refname --list "$currentTag" "$versionTag" | head -n 1)
    if isVersionTag "$currentTag" && [ "$highestTag" = "$currentTag" ] && [ "$currentTag" != "$versionTag" ]; then
        if [ -t 0 ]; then
            printf 'Downgrade Echo Build from %s to %s? [y/N] ' "$currentTag" "$versionTag"
            read -r answer
            case "$answer" in y|Y|yes|YES) ;;
                *) fail "downgrade cancelled" ;;
            esac
        else
            fail "non-interactive downgrade requires --allow-downgrade"
        fi
    fi
fi

printf '%s\n' "Building $versionTag from source (network and build output follow)..."
(cd "$sourceDir" && cargo build --locked --profile release-dist -p xai-grok-pager-bin --bin echo-build)

builtPath="$sourceDir/target/release-dist/echo-build"
[ -x "$builtPath" ] || fail "build completed without producing echo-build"
builtVersion=$($builtPath --version)
printf '%s\n' "$builtVersion"
printf '%s\n' "$builtVersion" | grep -F "echo-build $packageVersion (" >/dev/null || fail "built executable failed its identity, version, or source-revision check"

stagedPath="$installPath.new.$$"
previousPath="$installPath.previous"
cp "$builtPath" "$stagedPath"
chmod 755 "$stagedPath"

if [ -e "$installPath" ]; then
    cp "$installPath" "$previousPath"
fi

mv -f "$stagedPath" "$installPath"
if ! "$installPath" --version >/dev/null 2>&1; then
    if [ -x "$previousPath" ]; then
        mv -f "$previousPath" "$installPath"
    else
        rm -f "$installPath"
    fi
    fail "installed executable failed its startup check; the previous executable was restored"
fi

printf 'Installed %s at %s\n' "$versionTag" "$installPath"
case ":$PATH:" in
    *:"$installDir":*) ;;
    *) printf 'Add %s to PATH, for example: export PATH="%s:$PATH"\n' "$installDir" "$installDir" ;;
esac
