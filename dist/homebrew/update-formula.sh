#!/bin/sh
# Fill dist/homebrew/grove.rb with the version + per-target sha256 hashes from a
# published release, then print it. Copy the result to the tap repo
# (Entelligentsia/homebrew-grove) as Formula/grove.rb.
#
#   dist/homebrew/update-formula.sh v0.1.0 > grove.rb
set -eu

REPO="Entelligentsia/grove"
TAG="${1:?usage: update-formula.sh <vX.Y.Z>}"
VERSION="${TAG#v}"
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
TEMPLATE="$HERE/grove.rb"

base="https://github.com/$REPO/releases/download/$TAG"

hash_for() { # target -> bare sha256
  curl -fsSL "$base/grove-$1.tar.gz.sha256" | awk '{print $1}'
}

ad=$(hash_for aarch64-apple-darwin)
xd=$(hash_for x86_64-apple-darwin)
al=$(hash_for aarch64-unknown-linux-gnu)
xl=$(hash_for x86_64-unknown-linux-gnu)

sed \
  -e "s/^  version \".*\"/  version \"$VERSION\"/" \
  -e "s/REPLACE_AARCH64_APPLE_DARWIN/$ad/" \
  -e "s/REPLACE_X86_64_APPLE_DARWIN/$xd/" \
  -e "s/REPLACE_AARCH64_UNKNOWN_LINUX_GNU/$al/" \
  -e "s/REPLACE_X86_64_UNKNOWN_LINUX_GNU/$xl/" \
  "$TEMPLATE"
