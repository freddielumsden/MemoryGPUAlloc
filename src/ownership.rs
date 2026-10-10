use crate::allocator::Allocator;

struct Ownership {
    key: u32
}

impl Ownership {
    pub fn new(key: u32) -> Self {
        Ownership { key }
    }
}
