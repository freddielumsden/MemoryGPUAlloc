// Library entry point so other crates (e.g. the `ui/` web front-end) can use
// the backend. Mirrors the module list in `main.rs`; no logic lives here.
pub mod memory;
pub mod gpu;
pub mod group;
pub mod allocator;
