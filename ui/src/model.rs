//! UI-side workspace state.
//!
//! Everything in here owns *real* backend objects (`MemRef`, `MemPool`, `GPU`,
//! `Allocator`) and only moves them around through the backend's public API.
//! There is no egui code in this module: it is the glue between the backend
//! and the presentation layer in `app.rs`.

use gpu_memory_alloc::allocator::Allocator;
use gpu_memory_alloc::gpu::GPU;
use gpu_memory_alloc::memory::{MemPool, MemRef};

pub type NodeId = u64;

/// A `MemPool` placed in the workspace, tagged with a UI-only id.
pub struct PoolNode {
    pub id: NodeId,
    pub pool: MemPool,
}

pub enum AllocatorState {
    /// Being configured: GPUs and a pool can still be dragged in and out.
    Staging { gpus: Vec<GPU>, pool: Option<PoolNode> },
    /// `Allocator::new` has been called; configuration is locked.
    Online { allocator: Allocator },
}

pub struct AllocatorNode {
    pub id: NodeId,
    pub state: AllocatorState,
}

/// Where a `MemRef` can live in the workspace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RefLocation {
    Tray,
    Pool(NodeId),
}

/// Where a free (not yet owned by an online allocator) `GPU` can live.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GpuLocation {
    Rack,
    Allocator(NodeId),
}

/// Where a `MemPool` can live before being handed to `Allocator::new`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PoolLocation {
    Workspace,
    Allocator(NodeId),
}

pub type OpResult<T = ()> = Result<T, String>;

pub struct Workspace {
    next_pool_id: NodeId,
    next_allocator_id: NodeId,
    /// Loose memory references ("memory blocks") not yet in any pool.
    pub tray: Vec<MemRef>,
    pub pools: Vec<PoolNode>,
    /// GPUs not assigned to any allocator.
    pub rack: Vec<GPU>,
    pub allocators: Vec<AllocatorNode>,
}

impl Default for Workspace {
    /// Mirrors `main.rs`: 18 memory refs, 8 GPUs, one pool and one allocator,
    /// all left unwired so they can be assembled by hand.
    fn default() -> Self {
        let mut ws = Workspace::empty();
        ws.add_refs(18);
        ws.add_pools(1);
        ws.add_gpus(8);
        ws.add_allocators(1);
        ws
    }
}

impl Workspace {
    pub fn empty() -> Self {
        Workspace {
            next_pool_id: 1,
            next_allocator_id: 1,
            tray: Vec::new(),
            pools: Vec::new(),
            rack: Vec::new(),
            allocators: Vec::new(),
        }
    }

    fn fresh_id(counter: &mut NodeId) -> NodeId {
        let id = *counter;
        *counter += 1;
        id
    }

    // ---------------------------------------------------------------- add / remove

    pub fn add_refs(&mut self, n: usize) {
        self.tray.extend((0..n).map(|_| MemRef::new()));
    }

    /// Removes up to `n` loose refs from the tray. Returns how many were removed.
    pub fn remove_refs(&mut self, n: usize) -> usize {
        let n = n.min(self.tray.len());
        self.tray.truncate(self.tray.len() - n);
        n
    }

    pub fn add_pools(&mut self, n: usize) {
        for _ in 0..n {
            let id = Self::fresh_id(&mut self.next_pool_id);
            self.pools.push(PoolNode { id, pool: MemPool::new() });
        }
    }

    /// Removes up to `n` workspace pools (newest first). Their refs go back to
    /// the tray. Returns how many pools were removed.
    pub fn remove_pools(&mut self, n: usize) -> usize {
        let n = n.min(self.pools.len());
        for _ in 0..n {
            let node = self.pools.pop().expect("checked length");
            self.tray.extend(node.pool);
        }
        n
    }

    pub fn add_gpus(&mut self, n: usize) {
        self.rack.extend((0..n).map(|_| GPU::new()));
    }

    /// Removes up to `n` GPUs from the rack. Returns how many were removed.
    pub fn remove_gpus(&mut self, n: usize) -> usize {
        let n = n.min(self.rack.len());
        self.rack.truncate(self.rack.len() - n);
        n
    }

    pub fn add_allocators(&mut self, n: usize) {
        for _ in 0..n {
            let id = Self::fresh_id(&mut self.next_allocator_id);
            self.allocators.push(AllocatorNode { id, state: AllocatorState::Staging { gpus: Vec::new(), pool: None } });
        }
    }

    /// Removes up to `n` allocators (newest first). A staging allocator hands
    /// its GPUs and pool back to the workspace; an online allocator is dropped
    /// together with everything it owns, since the backend has no way to give
    /// them back. Returns how many allocators were removed.
    pub fn remove_allocators(&mut self, n: usize) -> usize {
        let n = n.min(self.allocators.len());
        for _ in 0..n {
            let node = self.allocators.pop().expect("checked length");
            if let AllocatorState::Staging { gpus, pool } = node.state {
                self.rack.extend(gpus);
                self.pools.extend(pool);
            }
        }
        n
    }

    // ---------------------------------------------------------------- lookups

    pub fn pool_location(&self, pool_id: NodeId) -> Option<PoolLocation> {
        if self.pools.iter().any(|p| p.id == pool_id) {
            return Some(PoolLocation::Workspace);
        }
        self.allocators.iter().find_map(|a| match &a.state {
            AllocatorState::Staging { pool: Some(p), .. } if p.id == pool_id => Some(PoolLocation::Allocator(a.id)),
            _ => None,
        })
    }

    fn pool_mut(&mut self, pool_id: NodeId) -> Option<&mut MemPool> {
        if let Some(p) = self.pools.iter_mut().find(|p| p.id == pool_id) {
            return Some(&mut p.pool);
        }
        self.allocators.iter_mut().find_map(|a| match &mut a.state {
            AllocatorState::Staging { pool: Some(p), .. } if p.id == pool_id => Some(&mut p.pool),
            _ => None,
        })
    }

    fn allocator_mut(&mut self, id: NodeId) -> OpResult<&mut AllocatorNode> {
        self.allocators.iter_mut().find(|a| a.id == id).ok_or_else(|| format!("allocator #{id} not found"))
    }

    fn staging_mut(&mut self, id: NodeId) -> OpResult<(&mut Vec<GPU>, &mut Option<PoolNode>)> {
        match &mut self.allocator_mut(id)?.state {
            AllocatorState::Staging { gpus, pool } => Ok((gpus, pool)),
            AllocatorState::Online { .. } => Err(format!("allocator #{id} is online; its configuration is locked")),
        }
    }

    fn online_mut(&mut self, id: NodeId) -> OpResult<&mut Allocator> {
        match &mut self.allocator_mut(id)?.state {
            AllocatorState::Online { allocator } => Ok(allocator),
            AllocatorState::Staging { .. } => Err(format!("allocator #{id} has not been initialised")),
        }
    }

    // ---------------------------------------------------------------- moves

    pub fn move_ref(&mut self, from: RefLocation, to: RefLocation) -> OpResult {
        if from == to {
            return Ok(());
        }
        if let RefLocation::Pool(id) = to {
            self.pool_mut(id).ok_or_else(|| format!("pool #{id} not found"))?;
        }
        let mem_ref = match from {
            RefLocation::Tray => self.tray.pop(),
            RefLocation::Pool(id) => self.pool_mut(id).and_then(MemPool::get_ref),
        }
        .ok_or("no memory ref available at the source")?;
        match to {
            RefLocation::Tray => self.tray.push(mem_ref),
            RefLocation::Pool(id) => self.pool_mut(id).expect("checked above").add_ref(mem_ref),
        }
        Ok(())
    }

    pub fn move_gpu(&mut self, from: GpuLocation, to: GpuLocation) -> OpResult {
        if from == to {
            return Ok(());
        }
        if let GpuLocation::Allocator(id) = to {
            self.staging_mut(id)?;
        }
        let gpu = match from {
            GpuLocation::Rack => self.rack.pop(),
            GpuLocation::Allocator(id) => self.staging_mut(id)?.0.pop(),
        }
        .ok_or("no GPU available at the source")?;
        match to {
            GpuLocation::Rack => self.rack.push(gpu),
            GpuLocation::Allocator(id) => self.staging_mut(id).expect("checked above").0.push(gpu),
        }
        Ok(())
    }

    pub fn move_pool(&mut self, pool_id: NodeId, to: PoolLocation) -> OpResult {
        let from = self.pool_location(pool_id).ok_or_else(|| format!("pool #{pool_id} not found"))?;
        if from == to {
            return Ok(());
        }
        if let PoolLocation::Allocator(id) = to {
            if self.staging_mut(id)?.1.is_some() {
                return Err(format!("allocator #{id} already has a memory pool"));
            }
        }
        let node = match from {
            PoolLocation::Workspace => {
                let idx = self.pools.iter().position(|p| p.id == pool_id).expect("located above");
                self.pools.remove(idx)
            }
            PoolLocation::Allocator(id) => self.staging_mut(id)?.1.take().expect("located above"),
        };
        match to {
            PoolLocation::Workspace => self.pools.push(node),
            PoolLocation::Allocator(id) => *self.staging_mut(id).expect("checked above").1 = Some(node),
        }
        Ok(())
    }

    // ---------------------------------------------------------------- allocator ops

    /// Calls `Allocator::new` with the staged GPUs and pool (an empty pool if
    /// none was staged). After this the configuration cannot change.
    pub fn initialise(&mut self, id: NodeId) -> OpResult {
        let (gpus, pool) = self.staging_mut(id)?;
        let gpus = std::mem::take(gpus);
        let pool = pool.take().map(|p| p.pool).unwrap_or_else(MemPool::new);
        self.allocator_mut(id)?.state = AllocatorState::Online { allocator: Allocator::new(gpus, pool) };
        Ok(())
    }

    pub fn allocate(&mut self, id: NodeId, size: usize, gpu_index: usize) -> OpResult {
        self.online_mut(id)?
            .allocate(size, gpu_index)
            .ok_or_else(|| format!("backend rejected allocate({size}, {gpu_index})"))
    }

    pub fn allocate_evenly(&mut self, id: NodeId, size: usize) -> OpResult {
        self.online_mut(id)?.allocate_evenly(size).ok_or_else(|| format!("backend rejected allocate_evenly({size})"))
    }

    pub fn deallocate(&mut self, id: NodeId, gpu_index: usize) -> OpResult {
        self.online_mut(id)?.deallocate(gpu_index).ok_or_else(|| format!("backend rejected deallocate({gpu_index})"))
    }

    pub fn run(&mut self, id: NodeId) -> OpResult {
        self.online_mut(id)?.run();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assemble_and_allocate_like_main() {
        let mut ws = Workspace::default();
        let pool_id = ws.pools[0].id;
        let alloc_id = ws.allocators[0].id;
        for _ in 0..18 {
            ws.move_ref(RefLocation::Tray, RefLocation::Pool(pool_id)).unwrap();
        }
        for _ in 0..8 {
            ws.move_gpu(GpuLocation::Rack, GpuLocation::Allocator(alloc_id)).unwrap();
        }
        ws.move_pool(pool_id, PoolLocation::Allocator(alloc_id)).unwrap();
        ws.initialise(alloc_id).unwrap();
        ws.allocate_evenly(alloc_id, 18).unwrap();
        ws.run(alloc_id).unwrap();

        let AllocatorState::Online { allocator } = &ws.allocators[0].state else { panic!() };
        let refs: Vec<_> = (0..allocator.no_gpus()).map(|i| allocator.gpu_ref_count(i).unwrap_or(0)).collect();
        assert_eq!(refs, vec![3, 3, 2, 2, 2, 2, 2, 2]);
        assert_eq!(allocator.pool_size(), 0);

        ws.deallocate(alloc_id, 0).unwrap();
        let AllocatorState::Online { allocator } = &ws.allocators[0].state else { panic!() };
        assert_eq!(allocator.pool_size(), 3);
        assert_eq!(allocator.gpu_ref_count(0), None);
        assert!(ws.allocate(alloc_id, 4, 1).is_err());
    }

    #[test]
    fn removing_staging_allocator_returns_parts() {
        let mut ws = Workspace::default();
        let alloc_id = ws.allocators[0].id;
        let pool_id = ws.pools[0].id;
        ws.move_gpu(GpuLocation::Rack, GpuLocation::Allocator(alloc_id)).unwrap();
        ws.move_pool(pool_id, PoolLocation::Allocator(alloc_id)).unwrap();
        ws.move_ref(RefLocation::Tray, RefLocation::Pool(pool_id)).unwrap();
        assert_eq!(ws.remove_allocators(5), 1);
        assert_eq!(ws.rack.len(), 8);
        assert_eq!(ws.pools.len(), 1);
        assert_eq!(ws.remove_pools(1), 1);
        assert_eq!(ws.tray.len(), 18);
    }
}
