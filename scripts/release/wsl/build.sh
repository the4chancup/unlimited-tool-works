#!/bin/bash
# Pack the Linux bundle of one commit in WSL's Ubuntu 22.04, check it, and copy it to the
# Windows side. `scripts/release/wsl_release.py` runs it as the build user; `setup.sh` makes
# that user.
#
# usage: build.sh <git dir> <commit> <version> <destination folder>
#
# The repository is cloned once into the Linux file system (a build on the Windows drive is far
# slower) and fetched from the Windows repository's git folder on each run.
set -euo pipefail
gitdir=$1 commit=$2 version=$3 dest=$4
# The release workflow's floor: a bundle asking for a newer glibc is refused.
GLIBC_FLOOR=2.35

. "$HOME/.cargo/env"
export PATH="$HOME/bin:$PATH" CARGO_BUILD_JOBS=4
name=unlimited-tool-works-$version-linux
if [ -e "$dest/$name.tar.gz" ]; then
    echo "error: $dest/$name.tar.gz already exists (a bundle is never overwritten)"
    exit 1
fi

dir=$HOME/build/utw
if [ ! -d "$dir/.git" ]; then
    mkdir -p "$(dirname "$dir")"
    git clone --quiet --no-checkout "$gitdir" "$dir"
fi
cd "$dir"
git fetch --quiet "$gitdir" '+refs/heads/*:refs/remotes/source/*'
git checkout --quiet --detach "$commit"
# The clone keeps no bundle: the copy on the Windows side is the one kept, and release.py
# refuses to write over an archive left by an earlier build of the same version.
rm -f "target/dist/$name"*
just release "$version"

cd target/dist
sha256sum -c "$name.tar.gz.sha256"
check=$(mktemp -d)
tar xzf "$name.tar.gz" -C "$check"
binary="$check/unlimited-tool-works-$version/utw"
reported=$("$binary" --version)
case "$reported" in *"v$version "*) ;; *) echo "error: utw --version reads '$reported', not $version"; exit 1;; esac
newest=$(objdump -T "$binary" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -uV | tail -n 1)
if [ "$(printf '%s\n%s\n' "$newest" "$GLIBC_FLOOR" | sort -V | tail -n 1)" != "$GLIBC_FLOOR" ]; then
    echo "error: the binary asks for glibc $newest, newer than the floor $GLIBC_FLOOR"
    exit 1
fi
rm -rf "$check"
mkdir -p "$dest"
cp "$name.tar.gz" "$name.tar.gz.sha256" "$name-symbols.tar.gz" "$dest/"
echo "$dest/$name.tar.gz: starts, glibc $newest at most"
