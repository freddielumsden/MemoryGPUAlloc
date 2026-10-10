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
- `src/widgets.rs`, `src/theme.rs` — reusable widgets and the dark, rounded theme.

Notes on current backend limits:

- `Allocator` has no getters, so the UI keeps a display-only mirror of pool size and per-GPU ref counts,
  updated only when a backend call succeeds.
- `group::Group` is private, so it is listed but cannot be created yet.
- `GPU::run` prints with `println!`, which goes nowhere in the browser; the event log records the call instead.
