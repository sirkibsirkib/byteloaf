use core::{ops::Range, sync::atomic};
use std::alloc;

#[cfg(test)]
mod tests;

/// The owner of a slice of heap-allocated bytes.
/// Can be split into a pair of byte-loaves,
/// and owned bytes can be accessed for reading and writing.
/// The underlying buffer is owned by all Byteloaves owning it's slices,
/// and is freed by the last of its Byteloaves to drop.
#[derive(Debug)]
pub struct LoafSlice {
    // invariants:
    // 1. `loaf_ptr` points to an allocation of (LoafHeader,X)
    //	  where X is some sequence of bytes such that the entire allocation has size header.alloc_size
    // 2. No other LoafHeader exists where (loaf_ptr+slice_range.start..loaf_ptr+slice_range.end) overlaps with mine.
    loaf_ptr: usize,
    slice_range: Range<usize>,
}
const USIZE_BYTES: usize = std::mem::size_of::<usize>();

struct LoafHeader {
    arc: atomic::AtomicUsize,
    alloc_size: usize,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ResplitError {
    DistinctLoaves,
    NotAdjacent,
}

const fn usize_bytes_round_down(x: usize) -> usize {
    x & !(USIZE_BYTES - 1)
}
const fn usize_bytes_round_up(x: usize) -> usize {
    usize_bytes_round_down(x + USIZE_BYTES - 1)
}
impl LoafSlice {
    pub const MAX_LOAF_LEN: usize = usize_bytes_round_down(isize::MAX as usize - (2 * USIZE_BYTES));
    fn capped_offset(&self, offset: usize) -> usize {
        let Range { start, end } = self.slice_range;
        start.saturating_add(offset).min(end)
    }

    pub fn new(loaf_len: usize) -> Self {
        if loaf_len > Self::MAX_LOAF_LEN {
            panic!("Can't support loaf of that size!")
        }
        let alloc_size = usize_bytes_round_up(loaf_len + 2 * USIZE_BYTES);
        let loaf_ptr = unsafe {
            // safe! alloc_size is multiple of USIZE_BYTES which is a power of two.
            let layout = alloc::Layout::from_size_align_unchecked(alloc_size, USIZE_BYTES);
            alloc::alloc(layout)
        };
        let header_ptr = loaf_ptr as *mut LoafHeader;
        unsafe {
            // Header structure is in allocated space, well-aligned, and uniquely accessed
            header_ptr.write(LoafHeader {
                arc: 1.into(),
                alloc_size,
            });
        }
        LoafSlice {
            loaf_ptr: loaf_ptr as usize,
            slice_range: 0..loaf_len,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }
    pub fn try_joined(mut self, mut other: Self) -> Result<Self, (ResplitError, [Self; 2])> {
        match self.try_join(&mut other) {
            Ok(()) => Ok(self),
            Err(resplit_error) => Err((resplit_error, [self, other])),
        }
    }
    pub fn try_join(&mut self, other: &mut Self) -> Result<(), ResplitError> {
        if self.slice_range.end <= other.slice_range.end {
            // self comes first;
            Self::try_resplit_at(self, other, other.slice_range.end)
        } else {
            // other comes first
            Self::try_resplit_at(other, self, other.slice_range.start)
        }
    }
    pub fn try_resplit_at(a: &mut Self, b: &mut Self, self_len: usize) -> Result<(), ResplitError> {
        if a.loaf_ptr != b.loaf_ptr {
            Err(ResplitError::DistinctLoaves)
        } else if a.slice_range.end != b.slice_range.start {
            Err(ResplitError::NotAdjacent)
        } else {
            let middle = a.capped_offset(self_len);
            a.slice_range.end = middle;
            b.slice_range.start = middle;
            Ok(())
        }
    }
    pub fn split_at(mut self, at: usize) -> [Self; 2] {
        let tail = self.split_after(at);
        [self, tail]
    }
    pub fn split_after(&mut self, self_len: usize) -> Self {
        let end = self.slice_range.end;
        let middle = self.capped_offset(self_len);
        self.slice_range.end = middle;

        let header_ptr = self.loaf_ptr as *mut LoafHeader;
        let header_ref: &LoafHeader = unsafe {
            // safe! pointer points to initialized LoafHeader value,
            // which is currently not mut accessed elsewhere
            &*header_ptr
        };
        let was = header_ref.arc.fetch_add(1, atomic::Ordering::SeqCst);
        if was > usize::MAX {
            std::process::abort();
        }

        Self {
            loaf_ptr: self.loaf_ptr,
            slice_range: middle..end,
        }
    }
    fn bytes_start(&self) -> *mut u8 {
        let ptr = self.loaf_ptr as *mut u8;
        let start_offset = core::mem::size_of::<LoafHeader>() + self.slice_range.start;
        unsafe {
            // safe! `add` relies on invariant to stay in bounds
            (ptr).add(start_offset)
        }
    }
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.bytes_start(), self.slice_range.len()) }
    }
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.bytes_start(), self.slice_range.len()) }
    }
}
impl Drop for LoafSlice {
    fn drop(&mut self) {
        let header_ptr = self.loaf_ptr as *mut LoafHeader;
        let header_ref: &LoafHeader = unsafe { &*header_ptr };
        let was = header_ref.arc.fetch_sub(1, atomic::Ordering::SeqCst);
        if was == 1 {
            // I am the final owner! drop!
            let layout = unsafe {
                // safe!
                alloc::Layout::from_size_align_unchecked(header_ref.alloc_size, USIZE_BYTES)
            };
            unsafe {
                // safe!
                alloc::dealloc(self.loaf_ptr as *mut u8, layout)
            }
        }
    }
}
impl AsRef<[u8]> for LoafSlice {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}
impl AsMut<[u8]> for LoafSlice {
    fn as_mut(&mut self) -> &mut [u8] {
        self.as_slice_mut()
    }
}
