use crate::memory::MemRef;
use rand::rngs::SmallRng;
use rand::RngExt;

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

    pub fn run(&mut self, memory: &mut Vec<MemRef>) {
        self.set_availability(false);

        for mem in memory {
            mem.map(randomise)
        }

        self.set_availability(true);
    }

}

fn randomise(_: u8) -> u8 {
    let mut rng: SmallRng = rand::make_rng();
    let value: u8 = rng.random_range(0..=255);

    value
}
