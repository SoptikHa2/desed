#!/bin/bash
# Run this and check changes.
# When ready, pass the --overwrite flag.

# Abort on error
set -eo pipefail

repo_root=$(git rev-parse --show-toplevel)
# Make sure we have the latest release
git pull

# Get the last release number
latest_version_published=$(git tag | tail -1 | sed 's/^v//')
latest_commit_hash=$(git log -1 --format="%h")
release="$latest_version_published.$latest_commit_hash"

# Replace pkgver and clone revision in pkgbuild
new_pkgbuild=$(sed -Ee '
/^pkgver=/ {
    s/.*//
    s/^/pkgver='"$release"'/
}
' <"$repo_root/aur/desed-git/PKGBUILD")

if [[ "$1" == "--overwrite" ]]; then
    echo "$new_pkgbuild" | sponge "$repo_root/aur/desed-git/PKGBUILD"
    (
        cd "$repo_root/aur/desed-git" || exit 1
        makepkg --printsrcinfo > ".SRCINFO"
        git add "PKGBUILD" ".SRCINFO"
        git commit -m "Bumped up upstream version"
        git push
    )
    echo "Done."
else
    echo "$new_pkgbuild"
fi

