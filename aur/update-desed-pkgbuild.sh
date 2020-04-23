#!/bin/bash
# Run this and check changes.
# When ready, pass the --overwrite flag.

# Abort on error
set -eo pipefail

repo_root=$(git rev-parse --show-toplevel)
# Make sure we have the latest release
git pull

# Get the last release number
release=$(git describe --tags | sed -E 's/v([^-]*)-.*/\1/')

# Replace pkgver and clone revision in pkgbuild
new_pkgbuild=$(sed -Ee '
/^pkgver=/ {
    s/.*//
    s/^/'"$release"'/
    h
    s/^/pkgver=/
}
/git checkout .tags\/v/ {
    G
    s/\n//
    s/tags\/v.*'"'"'/tags\/v/
    s/$/'"'"'/
}
' <"$repo_root/aur/desed/PKGBUILD")

if [[ "$1" == "--overwrite" ]]; then
    echo "$new_pkgbuild" | sponge "$repo_root/aur/desed/PKGBUILD"
    (
        cd "$repo_root/aur/desed" || exit 1
        makepkg --printsrcinfo > ".SRCINFO"
    )
    echo "Ready. Verify, commit and push."
else
    echo "$new_pkgbuild"
fi

