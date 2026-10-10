use crate::memory::MemRef;

pub struct GPU {
    is_available: bool,
}

impl GPU {
    pub fn new() -> Self {
        GPU { is_available: true }
    }

    pub fn is_available(&self) -> bool {
        self.is_available
    }

    pub fn set_availability(&mut self, available: bool) {
        self.is_available = available;
    }

    pub fn run(&mut self, memory: Vec<MemRef>) -> Vec<MemRef> {
        self.set_availability(false);
        println!("Running with {:?} memory references", memory.len());
        self.set_availability(true);

        memory
    }
}
