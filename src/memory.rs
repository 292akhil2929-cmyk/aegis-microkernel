//! Physical 4 KiB frame allocation with deterministic, fixed-size metadata.

pub const PAGE_SIZE: u64 = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Frame {
    pub physical_address: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameError {
    EmptyRegion,
    Unaligned,
    OutOfRange,
    OutOfMemory,
    DoubleFree,
}

/// A first-fit bitmap allocator. A set bit means allocated or reserved.
pub struct FrameAllocator<const WORDS: usize> {
    base: u64,
    frame_count: usize,
    used: [u64; WORDS],
    next_hint: usize,
}

impl<const WORDS: usize> FrameAllocator<WORDS> {
    pub fn new(base: u64, bytes: u64) -> Result<Self, FrameError> {
        if base % PAGE_SIZE != 0 || bytes % PAGE_SIZE != 0 {
            return Err(FrameError::Unaligned);
        }
        let frame_count = (bytes / PAGE_SIZE) as usize;
        if frame_count == 0 || frame_count > WORDS * 64 {
            return Err(FrameError::EmptyRegion);
        }
        let mut allocator = Self {
            base,
            frame_count,
            used: [0; WORDS],
            next_hint: 0,
        };
        for index in frame_count..WORDS * 64 {
            allocator.set_used(index, true);
        }
        Ok(allocator)
    }

    pub fn allocate(&mut self) -> Result<Frame, FrameError> {
        for offset in 0..self.frame_count {
            let index = (self.next_hint + offset) % self.frame_count;
            if !self.is_used(index) {
                self.set_used(index, true);
                self.next_hint = (index + 1) % self.frame_count;
                return Ok(Frame {
                    physical_address: self.base + index as u64 * PAGE_SIZE,
                });
            }
        }
        Err(FrameError::OutOfMemory)
    }

    pub fn reserve(&mut self, start: u64, pages: usize) -> Result<(), FrameError> {
        if start % PAGE_SIZE != 0 {
            return Err(FrameError::Unaligned);
        }
        let first = self.index_of(start)?;
        let end = first.checked_add(pages).ok_or(FrameError::OutOfRange)?;
        if end > self.frame_count {
            return Err(FrameError::OutOfRange);
        }
        for index in first..end {
            self.set_used(index, true);
        }
        Ok(())
    }

    pub fn free(&mut self, frame: Frame) -> Result<(), FrameError> {
        let index = self.index_of(frame.physical_address)?;
        if !self.is_used(index) {
            return Err(FrameError::DoubleFree);
        }
        self.set_used(index, false);
        if index < self.next_hint {
            self.next_hint = index;
        }
        Ok(())
    }

    pub fn available(&self) -> usize {
        (0..self.frame_count)
            .filter(|index| !self.is_used(*index))
            .count()
    }

    fn index_of(&self, address: u64) -> Result<usize, FrameError> {
        if address % PAGE_SIZE != 0 || address < self.base {
            return Err(FrameError::OutOfRange);
        }
        let index = ((address - self.base) / PAGE_SIZE) as usize;
        if index >= self.frame_count {
            Err(FrameError::OutOfRange)
        } else {
            Ok(index)
        }
    }

    fn is_used(&self, index: usize) -> bool {
        self.used[index / 64] & (1u64 << (index % 64)) != 0
    }

    fn set_used(&mut self, index: usize, used: bool) {
        let mask = 1u64 << (index % 64);
        if used {
            self.used[index / 64] |= mask;
        } else {
            self.used[index / 64] &= !mask;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocation_skips_reserved_frames_and_reuses_freed_frames() {
        let mut allocator = FrameAllocator::<1>::new(0x4000_0000, 8 * PAGE_SIZE).unwrap();
        allocator.reserve(0x4000_0000, 2).unwrap();
        let first = allocator.allocate().unwrap();
        let second = allocator.allocate().unwrap();
        assert_eq!(first.physical_address, 0x4000_2000);
        assert_eq!(second.physical_address, 0x4000_3000);
        allocator.free(first).unwrap();
        assert_eq!(allocator.allocate().unwrap(), first);
    }

    #[test]
    fn exhaustion_and_double_free_are_detected() {
        let mut allocator = FrameAllocator::<1>::new(0x8000, PAGE_SIZE).unwrap();
        let frame = allocator.allocate().unwrap();
        assert_eq!(allocator.allocate(), Err(FrameError::OutOfMemory));
        allocator.free(frame).unwrap();
        assert_eq!(allocator.free(frame), Err(FrameError::DoubleFree));
    }

    #[test]
    fn invalid_ranges_are_rejected() {
        let mut allocator = FrameAllocator::<1>::new(0x1000, 4 * PAGE_SIZE).unwrap();
        assert_eq!(allocator.reserve(0x1001, 1), Err(FrameError::Unaligned));
        assert_eq!(allocator.reserve(0x5000, 1), Err(FrameError::OutOfRange));
    }
}
