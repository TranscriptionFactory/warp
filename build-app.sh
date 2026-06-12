#!/bin/bash
set -e

cd "$(dirname "$0")"

DEBUG=0
for arg in "$@"; do
    [[ "$arg" == "--debug" ]] && DEBUG=1
done

echo "Building OpenWarp..."
# The bundle script's final create-dmg step fails on recent macOS due to
# hdiutil mount detection. We only need the .app, so tolerate that failure
# and verify the .app exists below.
if [[ $DEBUG -eq 1 ]]; then
    ./script/macos/bundle --channel oss --nosign --nouniversal --debug -o || true
    APP="target/aarch64-apple-darwin/debug/bundle/osx/OpenWarp.app"
else
    ./script/macos/bundle --channel oss --nosign --nouniversal -o || true
    APP="target/aarch64-apple-darwin/release-lto/bundle/osx/OpenWarp.app"
fi

# Copy to ~/Applications (user-writable, no admin/sudo required)
if [ -d "$APP" ]; then
    DEST="$HOME/Applications"
    echo "Installing to $DEST..."
    mkdir -p "$DEST"
    rm -rf "$DEST/OpenWarp.app"
    cp -R "$APP" "$DEST/"
    echo "Done! OpenWarp is now in $DEST."
else
    echo "Build failed — .app not found at $APP"
    exit 1
fi
