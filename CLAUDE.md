# Project notes for Claude

- `src/` is the backend (owned by the repo author); `ui/` is the egui web front-end that uses it as a library.
- The live site is GitHub Pages serving the committed build in `docs/` (default "Deploy from a branch" setup).

## Always rebuild `docs/` after a change

Whenever you change anything that affects the web app (`ui/`, `src/`, `Cargo.toml`/`Cargo.lock`), rebuild the site
and commit the updated `docs/` in the same commit as the change:

```sh
ui/build-pages.sh
```

The script needs `trunk` and the `wasm32-unknown-unknown` target
(`rustup target add wasm32-unknown-unknown`; `cargo install --locked trunk` or a prebuilt trunk release binary).
Old hashed files in `docs/` are replaced by each build — commit deletions too (`git add -A docs`).
