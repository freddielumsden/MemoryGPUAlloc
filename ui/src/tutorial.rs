//! First-run walkthrough. Decides *what* should be highlighted at each step;
//! `app.rs` reports *where* those things are drawn and paints the outlines.

use crate::model::{AllocatorNode, AllocatorState, NodeId, PoolNode, Workspace};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    RefToPool,
    PoolToAllocator,
    GpuToAllocator,
    Initialise,
    Allocate,
    Run,
    Done,
}

impl Step {
    const ORDER: [Step; 6] =
        [Step::RefToPool, Step::PoolToAllocator, Step::GpuToAllocator, Step::Initialise, Step::Allocate, Step::Run];

    fn next(self) -> Step {
        match self {
            Step::RefToPool => Step::PoolToAllocator,
            Step::PoolToAllocator => Step::GpuToAllocator,
            Step::GpuToAllocator => Step::Initialise,
            Step::Initialise => Step::Allocate,
            Step::Allocate => Step::Run,
            Step::Run | Step::Done => Step::Done,
        }
    }
}

/// A highlightable place on screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spot {
    FirstTrayRef,
    /// The drop area inside a pool card.
    PoolInset(NodeId),
    /// The whole pool card (as something to drag).
    PoolCard(NodeId),
    AllocPoolSlot(NodeId),
    FirstRackGpu,
    AllocGpuSlot(NodeId),
    Initialise(NodeId),
    Allocate(NodeId),
    Run(NodeId),
}

pub struct Tutorial {
    step: Step,
    visible: bool,
    ran: bool,
}

impl Default for Tutorial {
    fn default() -> Self {
        Tutorial { step: Step::RefToPool, visible: true, ran: false }
    }
}

impl Tutorial {
    pub fn step(&self) -> Step {
        self.step
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// 1-based position of the current step, and the number of steps.
    pub fn progress(&self) -> (usize, usize) {
        let total = Step::ORDER.len();
        (Step::ORDER.iter().position(|s| *s == self.step).map_or(total, |i| i + 1), total)
    }

    pub fn text(&self) -> (&'static str, &'static str) {
        match self.step {
            Step::RefToPool => ("Fill a pool", "Drag a memory block from the top strip into a memory pool."),
            Step::PoolToAllocator => {
                ("Give the allocator a pool", "Drag the pool by its header badge into the allocator's pool slot.")
            }
            Step::GpuToAllocator => ("Attach a GPU", "Drag a GPU from the rack into the allocator's GPU slot."),
            Step::Initialise => ("Initialise", "Lock the configuration by calling Allocator::new()."),
            Step::Allocate => (
                "Allocate memory",
                "Hand refs from the allocator's pool to a GPU: press Allocate, or drag the pool tile onto a GPU.",
            ),
            Step::Run => ("Run", "Run the allocator; each GPU rewrites its refs and their colours change."),
            Step::Done => ("Walkthrough complete", "You have assembled and run an allocator."),
        }
    }

    pub fn note_run(&mut self) {
        self.ran = true;
    }

    /// Moves forward past every step the workspace already satisfies.
    pub fn update(&mut self, ws: &Workspace) {
        while self.step != Step::Done && self.is_complete(ws) {
            self.step = self.step.next();
        }
    }

    fn is_complete(&self, ws: &Workspace) -> bool {
        let online = ws.allocators.iter().any(|a| matches!(a.state, AllocatorState::Online { .. }));
        match self.step {
            Step::RefToPool => {
                online
                    || ws.pools.iter().any(|p| !p.pool.is_empty())
                    || ws.allocators.iter().filter_map(staged_pool).any(|p| !p.pool.is_empty())
            }
            Step::PoolToAllocator => online || ws.allocators.iter().any(|a| staged_pool(a).is_some()),
            Step::GpuToAllocator => {
                online
                    || ws.allocators.iter().any(
                        |a| matches!(&a.state, AllocatorState::Staging { gpus, pool: Some(_) } if !gpus.is_empty()),
                    )
            }
            Step::Initialise => online,
            Step::Allocate => ws.allocators.iter().any(|a| match &a.state {
                AllocatorState::Online { allocator } => {
                    (0..allocator.no_gpus()).any(|gpu| allocator.gpu_ref_count(gpu).is_some_and(|n| n > 0))
                }
                AllocatorState::Staging { .. } => false,
            }),
            Step::Run => self.ran,
            Step::Done => true,
        }
    }

    /// What to outline right now.
    pub fn spots(&self, ws: &Workspace) -> Vec<Spot> {
        if !self.visible {
            return Vec::new();
        }
        let alloc = target_allocator(ws);
        let mut spots = Vec::new();
        match self.step {
            Step::RefToPool => {
                if !ws.tray.is_empty() {
                    spots.push(Spot::FirstTrayRef);
                }
                if let Some(p) = ws.pools.first() {
                    spots.push(Spot::PoolInset(p.id));
                }
            }
            Step::PoolToAllocator => {
                let pool = ws.pools.iter().find(|p| !p.pool.is_empty()).or(ws.pools.first());
                spots.extend(pool.map(|p| Spot::PoolCard(p.id)));
                spots.extend(alloc.map(Spot::AllocPoolSlot));
            }
            Step::GpuToAllocator => {
                if !ws.rack.is_empty() {
                    spots.push(Spot::FirstRackGpu);
                }
                spots.extend(alloc.map(Spot::AllocGpuSlot));
            }
            Step::Initialise => spots.extend(alloc.map(Spot::Initialise)),
            Step::Allocate | Step::Run => {
                let online = ws.allocators.iter().find(|a| matches!(a.state, AllocatorState::Online { .. }));
                let spot = if self.step == Step::Allocate { Spot::Allocate } else { Spot::Run };
                spots.extend(online.map(|a| spot(a.id)));
            }
            Step::Done => {}
        }
        spots
    }
}

fn staged_pool(a: &AllocatorNode) -> Option<&PoolNode> {
    match &a.state {
        AllocatorState::Staging { pool, .. } => pool.as_ref(),
        AllocatorState::Online { .. } => None,
    }
}

/// The staging allocator the walkthrough is building: the first one that
/// already has a pool, otherwise the first staging allocator.
fn target_allocator(ws: &Workspace) -> Option<NodeId> {
    let staging = || ws.allocators.iter().filter(|a| matches!(a.state, AllocatorState::Staging { .. }));
    staging()
        .find(|a| matches!(a.state, AllocatorState::Staging { pool: Some(_), .. }))
        .or_else(|| staging().next())
        .map(|a| a.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{GpuLocation, PoolLocation, RefLocation};

    #[test]
    fn walks_through_every_step() {
        let mut ws = Workspace::default();
        let mut t = Tutorial::default();
        let (pool, alloc) = (ws.pools[0].id, ws.allocators[0].id);

        t.update(&ws);
        assert_eq!(t.step(), Step::RefToPool);
        assert_eq!(t.spots(&ws), vec![Spot::FirstTrayRef, Spot::PoolInset(pool)]);

        ws.move_ref(RefLocation::Tray, RefLocation::Pool(pool)).unwrap();
        t.update(&ws);
        assert_eq!(t.spots(&ws), vec![Spot::PoolCard(pool), Spot::AllocPoolSlot(alloc)]);

        ws.move_pool(pool, PoolLocation::Allocator(alloc)).unwrap();
        t.update(&ws);
        assert_eq!(t.spots(&ws), vec![Spot::FirstRackGpu, Spot::AllocGpuSlot(alloc)]);

        ws.move_gpu(GpuLocation::Rack, GpuLocation::Allocator(alloc)).unwrap();
        t.update(&ws);
        assert_eq!(t.spots(&ws), vec![Spot::Initialise(alloc)]);

        ws.initialise(alloc).unwrap();
        t.update(&ws);
        assert_eq!(t.spots(&ws), vec![Spot::Allocate(alloc)]);

        ws.allocate(alloc, 1, 0).unwrap();
        t.update(&ws);
        assert_eq!(t.spots(&ws), vec![Spot::Run(alloc)]);

        t.note_run();
        t.update(&ws);
        assert_eq!(t.step(), Step::Done);
        assert!(t.spots(&ws).is_empty());
    }
}
