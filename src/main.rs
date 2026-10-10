pub mod memory;
pub mod gpu;
pub mod group;
pub mod allocator;

use memory::{MemRef, MemPool};
use gpu::GPU;
use allocator::Allocator;

fn main() {
    let no_gpus = 8;
    let mut gpus = Vec::new();
    for _ in 0..no_gpus {
        gpus.push(GPU::new());
    }

    let no_refs = 18; 
    let mut mem_refs = Vec::new();
    for _ in 0..no_refs {
        mem_refs.push(MemRef::new());
    }

    let mut mem_pool = MemPool::new();
    mem_pool.add_refs(mem_refs);

    let mut allocator = Allocator::new(gpus, mem_pool);
    allocator.allocate_evenly(18).expect("Allocation failed");

    allocator.run();
}
