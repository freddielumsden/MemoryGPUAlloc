use std::sync::Mutex;

const MEMORY_SIZE: usize = 4;

struct Memory {
    data: Vec<u8>,
}

// A reference to a sizeable unit of memory, which is given to a GPU
pub struct MemRef {
    memory: Mutex<Memory>,
}

pub struct MemPool {
    pool: Vec<MemRef>,
}

impl Memory {
    pub fn new(length: usize) -> Self {
        Memory {
            data: vec![0; length],
        }
    }
    pub fn length(&self) -> usize {
        self.data.len()
    }
}

impl MemRef {
    pub fn new() -> Self {
        MemRef {
            memory: Mutex::new(Memory::new(MEMORY_SIZE)),
        }
    }

    pub fn length(&self) -> usize {
        let memory = self.memory.lock().expect("failed to access memory");
        memory.length()
    }

    pub fn read(&self, index: usize) -> Option<u8> {
        let memory = self.memory.lock().expect("failed to access memory");

        if index < memory.data.len() {
            Some(memory.data[index])
        } else {
            None
        }
    }

    pub fn write(&mut self, index: usize, value: u8) -> Result<(), &'static str> {
        let mut memory = self.memory.lock().expect("failed to access memory");
        let length = memory.data.len();
        if index < length {
            memory.data[index] = value;
            Ok(())
        } else {
            Err("Index out of bounds")
        }
    }

    pub fn map(&mut self, f: impl Fn(u8) -> u8) {
        let mut memory = self.memory.lock().expect("failed to access memory");
        for byte in &mut memory.data {
            *byte = f(*byte);
        }
    }

    pub fn fill(&mut self, value: u8) {
        let mut memory = self.memory.lock().expect("failed to access memory");
        for byte in &mut memory.data {
            *byte = value;
        }
    }

}

impl MemPool {
    pub fn new() -> Self {
        MemPool { pool: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.pool.is_empty()
    }

    pub fn size(&self) -> usize {
        self.pool.len()
    }

    pub fn add_ref(&mut self, mem_ref: MemRef) {
        self.pool.push(mem_ref);
    }

    pub fn add_refs(&mut self, mem_refs: Vec<MemRef>) {
        self.pool.extend(mem_refs);
    }

    pub fn get_ref(&mut self) -> Option<MemRef> {
        self.pool.pop()
    }

    pub fn get_refs(&mut self, count: usize) -> Option<Vec<MemRef>> {
        if count > self.size() {
            return None;
        }
        let mut mems = Vec::new();
        for _ in 0..count {
            mems.push(self.get_ref().expect("there should be at least as many mems as requested"));
        }

        Some(mems)
    }
}

impl Iterator for MemPool {
    type Item = MemRef;

    fn next(&mut self) -> Option<Self::Item> {
        self.pool.pop()
    }
}
