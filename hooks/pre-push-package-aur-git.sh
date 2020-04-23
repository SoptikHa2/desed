#!/bin/bash
set -eo pipefail

if [[ "$(git config user.email)" == "petr.stastny01@gmail.com" ]]; then
    (
        repo_root=$(git rev-parse --show-toplevel)
        "$repo_root/aur/update-desed-git-pkgbuild.sh"
        echo "Press <C-c> to abort pushing new AUR package version"
        "$repo_root/aur/update-desed-git-pkgbuild.sh" --overwrite
    )
fi

exit 0
