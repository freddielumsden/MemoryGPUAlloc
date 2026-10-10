use crate::gpu::GPU;
use crate::memory::{MemPool, MemRef};
use std::collections::HashMap;

pub struct Allocator {
    gpus: Vec<GPU>, // Can't change after init
    pool: MemPool,
    map: HashMap<usize, Vec<MemRef>>,
}

impl Allocator {
    pub fn new(gpus: Vec<GPU>, memory_pool: MemPool) -> Self {
        Allocator { gpus, pool: memory_pool, map: HashMap::new() }
    }

    pub fn pool_size(&self) -> usize {
        self.pool.size()
    }

    pub fn no_gpus(&self) -> usize {
        self.gpus.len()
    }

    pub fn gpu_ref_count(&self, gpu_index: usize) -> Option<usize> {
        self.map.get(&gpu_index).map(|v| v.len())
    }

    pub fn allocate(&mut self, size: usize, gpu_index: usize) -> Option<()> {
        if gpu_index >= self.gpus.len()
        || size > self.pool.size() {
            return None;
        }

        let mem_refs = self.pool.get_refs(size)?;

        self.map.entry(gpu_index).or_insert_with(Vec::new).extend(mem_refs);

        Some(())
    }

    pub fn allocate_or_panic(&mut self, size: usize, gpu_index: usize) -> () {
        if self.allocate(size, gpu_index).is_none() {
            panic!("Failed to allocate memory of size {} on GPU {}", size, gpu_index);
        }
    }

    pub fn allocate_evenly(&mut self, size: usize) -> Option<()> {
        let num_gpus = self.gpus.len();
        if num_gpus == 0 || size > self.pool.size() {
            return None;
        }

        let size_per_gpu = size / num_gpus;
        let mut remainder = size % num_gpus;

        for gpu_index in 0..num_gpus {
            if remainder > 0 {
                self.allocate(size_per_gpu + 1, gpu_index)?;
                remainder -= 1;
            } else {
                self.allocate(size_per_gpu, gpu_index)?;
            }
        }

        Some(())
    }

    pub fn deallocate(&mut self, gpu_index: usize) -> Option<()> {
        if gpu_index >= self.gpus.len() {
            return None;
        }

        if let Some(mem_refs) = self.map.remove(&gpu_index) {
            self.pool.add_refs(mem_refs);
            Some(())
        } else {
            None
        }
    }

    pub fn run(&mut self) {
        for gpu_index in 0..self.gpus.len() {
            if let Some(mem_refs) = self.map.get_mut(&gpu_index) {
                self.gpus[gpu_index].run(mem_refs);
            }
        }
    }
}
