use crate::memory::MemRef;

enum GroupType {
    PreFill,
    MidFill,
    Decode,
}

struct Group {
    id: u32,
    r#type: GroupType,
    mem_refs: Vec<MemRef>,
}

static mut GROUP_ID_COUNTER: u32 = 0;
impl Group {
    pub fn new(group_type: GroupType) -> Self {
        unsafe {
            GROUP_ID_COUNTER += 1;
            Group { id: GROUP_ID_COUNTER, r#type: group_type, mem_refs: Vec::new()}
        }
    }

    fn add_mem_ref(&mut self, mem_ref: MemRef) {
        self.mem_refs.push(mem_ref);
    }

    fn get_mem_refs(&self) -> &Vec<MemRef> {
        &self.mem_refs
    }

    fn get_id(&self) -> u32 {
        self.id
    }

    fn get_type(&self) -> &GroupType {
        &self.r#type
    }
}
