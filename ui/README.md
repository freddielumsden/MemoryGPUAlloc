# GPU Memory Allocator — web UI

An [egui](https://github.com/emilk/egui) front-end for the `gpu_memory_alloc` backend in the parent
directory. It uses the backend as a library (via `../src/lib.rs`) and does not change any backend code.

## Run in the browser

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
cd ui
trunk serve --open          # dev server on http://127.0.0.1:8080
```

`trunk build --release` writes a static site to `ui/dist/` that can be hosted anywhere.

## Host on GitHub Pages

The built site is committed in `docs/`, so the default Pages setup works with no Actions:

1. Settings → Pages → *Build and deployment* → Source: **Deploy from a branch**.
2. Branch: `main`, folder: **`/docs`** (or `/ (root)` — the root `index.html` redirects to `docs/`).

After changing the UI or backend, rebuild and commit `docs/`:

```sh
ui/build-pages.sh   # trunk build --release --public-url ./ --dist ../docs
```

## Run natively (optional)

```sh
cd ui
cargo run --release
```

## How it works

| Area | Backend type | Interaction |
|------|--------------|-------------|
| Memory blocks (top) | loose `MemRef`s | drag a block into a pool |
| Memory pools | `MemPool` | drop refs in / drag refs out; drag the pool badge into an allocator |
| GPU rack | `GPU` | drag GPUs into an allocator |
| Allocators | `Allocator` | **Staging**: collect GPUs + one pool, then *Initialise* (`Allocator::new`). **Online**: drag the internal-pool tile onto a GPU (`allocate(1, gpu)`), or use *Allocate*, *Allocate evenly*, *Deallocate*, *Run* |
| Components (right) | all of the above | add / remove N of each |

Code layout:

- `src/model.rs` — owns the real backend objects and moves them via the backend's public API. No egui.
- `src/app.rs` — panels, drag-and-drop rules, and the command queue applied after each frame.
- `src/tutorial.rs` — the first-run walkthrough: which step you're on and what to highlight.
- `src/widgets.rs`, `src/theme.rs` — reusable widgets (including the orbiting tutorial outline) and the
  light/dark palettes. Light is the default; toggle it from the header.

Notes:

- Online allocators are read live through `Allocator::pool_size`, `no_gpus` and `gpu_ref_count`.
- `GPU::run` prints with `println!`, which goes nowhere in the browser; the event log records the call instead.
