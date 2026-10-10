#!/usr/bin/env sh
# Builds the web UI into ../docs for GitHub Pages ("Deploy from a branch").
# Asset URLs are relative, so the site works under /<repo>/ as well as at a domain root.
set -e
cd "$(dirname "$0")"
trunk build --release --public-url ./ --dist ../docs
touch ../docs/.nojekyll
