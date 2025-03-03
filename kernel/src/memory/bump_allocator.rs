use super::PhysicalPageNumber;

pub struct BumpAllocator {
    start_inclusive: PhysicalPageNumber,
    end_inclusive: PhysicalPageNumber,
    next: PhysicalPageNumber,
}

impl BumpAllocator {
    pub const fn new(start_inclusive: PhysicalPageNumber, end_inclusive: PhysicalPageNumber) -> Self {
        Self {
            start_inclusive,
            end_inclusive,
            next: start_inclusive,
        }
    }

    pub fn allocate(&mut self) -> Option<PhysicalPageNumber> {
        if self.next <= self.end_inclusive {
            let ppn = self.next;
            self.next.0 += 1;
            
            Some(ppn)
        } else {
            None
        }
    }
}