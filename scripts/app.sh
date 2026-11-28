#!/bin/sh
# Build and install Hutarka (the menu bar app) and the `hutarka` CLI.
#
#   scripts/app.sh build      build the signed Hutarka.app
#   scripts/app.sh install    build, quit the running copy, replace ~/Applications/Hutarka.app, start it
#   scripts/app.sh cli        install the `hutarka` CLI into ~/.cargo/bin
#   scripts/app.sh all        install + cli
#
# APP_DEST overrides where `install` puts the app (default ~/Applications/Hutarka.app).
# NO_LAUNCH=1 skips starting the app after `install`.
set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_DIR="$ROOT/app"
BUNDLE="$ROOT/target/release/bundle/macos/Hutarka.app"
DEST="${APP_DEST:-$HOME/Applications/Hutarka.app}"
IDENTIFIER=dev.hutarka.app
SIGNING_IDENTITY="Hutarka Local"

PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
export PATH

die() { echo "$@" >&2; exit 1; }

check_tools() {
    command -v cargo >/dev/null 2>&1 || die "cargo not found. Install Rust, or put ~/.cargo/bin on PATH."
    command -v npm >/dev/null 2>&1 || die "npm not found. Install Node.js (brew install node)."
    # Without the stable identity macOS forgets the Accessibility permission on every build.
    security find-identity -v -p codesigning | grep -q "\"$SIGNING_IDENTITY\"" \
        || die "No valid \"$SIGNING_IDENTITY\" code-signing certificate. Run scripts/make-signing-cert.sh first."
}

build() {
    check_tools
    cd "$APP_DIR"
    [ -d node_modules ] || npm install
    npx tauri build --bundles app
    codesign --verify --strict "$BUNDLE" || die "the built app's signature does not verify"
}

# Refuse to delete anything that is not our own bundle: the identifier proves
# it is the one we put there.
assert_ours() {
    [ -e "$1" ] || return 0
    [ -d "$1" ] || die "$1 exists and is not an app bundle. Refusing to replace it."
    found=$(/usr/libexec/PlistBuddy -c "Print :CFBundleIdentifier" "$1/Contents/Info.plist" 2>/dev/null || true)
    [ "$found" = "$IDENTIFIER" ] || die "$1 has identifier '${found:-none}', not $IDENTIFIER. Refusing to replace it."
}

# A running copy would keep the deleted bundle alive: stop it first.
stop_running() {
    pgrep -f "Hutarka.app/Contents/MacOS/hutarka-app" >/dev/null 2>&1 || return 0
    osascript -e "quit app id \"$IDENTIFIER\"" >/dev/null 2>&1 || true
    i=0
    while pgrep -f "Hutarka.app/Contents/MacOS/hutarka-app" >/dev/null 2>&1; do
        i=$((i + 1))
        # A menu bar app may ignore the quit request: end it after 3 s.
        [ "$i" -eq 12 ] && pkill -TERM -f "Hutarka.app/Contents/MacOS/hutarka-app" || true
        [ "$i" -gt 40 ] && die "the running app did not quit; quit it from the menu bar and try again"
        sleep 0.25
    done
}

# The new copy is prepared and checked next to the destination first; the
# installed app is swapped out only once the copy is known good.
install_app() {
    build
    mkdir -p "$(dirname "$DEST")"
    [ "$(cd "$(dirname "$DEST")" && pwd -P)/$(basename "$DEST")" != "$(cd "$(dirname "$BUNDLE")" && pwd -P)/$(basename "$BUNDLE")" ] \
        || die "APP_DEST is the build output itself ($BUNDLE). Refusing to replace it."
    assert_ours "$DEST"
    NEW="$DEST.new"
    OLD="$DEST.old"
    assert_ours "$NEW"
    assert_ours "$OLD"
    rm -rf "$NEW" "$OLD"
    cp -R "$BUNDLE" "$NEW" || { rm -rf "$NEW"; die "cannot copy the app to $NEW; the installed app is untouched"; }
    codesign --verify --strict "$NEW" || { rm -rf "$NEW"; die "the copied app's signature does not verify; the installed app is untouched"; }
    stop_running
    [ -e "$DEST" ] && mv "$DEST" "$OLD"
    if ! mv "$NEW" "$DEST"; then
        [ -e "$OLD" ] && mv "$OLD" "$DEST"
        die "cannot move the new app into place; the old one is back"
    fi
    rm -rf "$OLD"
    echo
    echo "installed: $DEST"
    if [ "${NO_LAUNCH:-}" != "1" ]; then
        open "$DEST"
        echo "started"
    fi
}

install_cli() {
    command -v cargo >/dev/null 2>&1 || die "cargo not found. Install Rust, or put ~/.cargo/bin on PATH."
    cargo install --path "$ROOT/crates/hutarka-cli" --locked
    echo
    echo "installed: $(command -v hutarka)"
}

case "${1:-build}" in
    build) build; echo; echo "built: $BUNDLE" ;;
    install) install_app ;;
    cli) install_cli ;;
    all) install_app; install_cli ;;
    *) die "usage: $0 [build|install|cli|all]" ;;
esac
