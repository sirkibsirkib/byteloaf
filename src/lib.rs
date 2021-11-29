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
pub struct LoafPart {
    // invariants:
    // 1. `header_ptr` points to an allocation of (LoafHeader,X)
    //	  where X is some sequence of bytes such that the entire allocation has size header.alloc_size
    // 2. No other LoafHeader exists where (ptr_range.start..ptr_range.end) overlaps with mine.
    header_ptr: usize,
    ptr_range: Range<usize>,
}
const USIZE_BYTES: usize = std::mem::size_of::<usize>();

struct LoafHeader {
    arc: atomic::AtomicUsize,
    alloc_size: usize,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ResplitError {
    DistinctLoaves,
    PartsAreNotAdjacent,
    OutOfBounds,
}
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum JoinError {
    DistinctLoaves,
    PartsAreNotAdjacent,
}

const fn usize_bytes_round_down(x: usize) -> usize {
    x & !(USIZE_BYTES - 1)
}
const fn usize_bytes_round_up(x: usize) -> usize {
    usize_bytes_round_down(x + USIZE_BYTES - 1)
}
impl LoafPart {
    pub const MAX_LOAF_LEN: usize = usize_bytes_round_down(isize::MAX as usize - (2 * USIZE_BYTES));

    pub fn new_from_slice(slice: &[u8]) -> Self {
        let mut me = Self::new(slice.len());
        use std::io::Write;
        me.as_slice_mut().write_all(slice).unwrap();
        me
    }

    pub fn new(loaf_len: usize) -> Self {
        if loaf_len > Self::MAX_LOAF_LEN {
            panic!("Can't support loaf of that size!")
        }
        let alloc_size = usize_bytes_round_up(loaf_len + 2 * USIZE_BYTES);
        let header_ptr = unsafe {
            // safe! alloc_size is multiple of USIZE_BYTES which is a power of two.
            let layout = alloc::Layout::from_size_align_unchecked(alloc_size, USIZE_BYTES);
            alloc::alloc(layout)
        };
        let header_ptr = header_ptr as *mut LoafHeader;
        unsafe {
            // Header structure is in allocated space, well-aligned, and uniquely accessed
            let arc = 1.into();
            header_ptr.write(LoafHeader { arc, alloc_size });
        }
        let ptr_range_start = header_ptr as usize + core::mem::size_of::<LoafHeader>();
        LoafPart {
            header_ptr: header_ptr as usize,
            ptr_range: ptr_range_start..(ptr_range_start + loaf_len),
        }
    }
    pub fn try_set_relative_range(
        &mut self,
        mut new_relative_range: Range<usize>,
    ) -> Result<(), ()> {
        // correct edge case
        new_relative_range.end = new_relative_range.start.max(new_relative_range.end);

        if let Some(new_ptr_end) = new_relative_range.end.checked_add(self.ptr_range.start) {
            // no need to check start
            let new_ptr_start = new_relative_range.start + self.ptr_range.start;
            self.ptr_range = new_ptr_start..new_ptr_end;
            Ok(())
        } else {
            Err(())
        }
    }
    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }
    pub fn try_join(&mut self, other: &mut Self) -> Result<(), JoinError> {
        if self.header_ptr != other.header_ptr {
            Err(JoinError::DistinctLoaves)
        } else if self.ptr_range.end != other.ptr_range.start {
            Err(JoinError::PartsAreNotAdjacent)
        } else {
            self.ptr_range.end = other.ptr_range.end;
            other.ptr_range.start = other.ptr_range.end;
            Ok(())
        }
    }
    pub fn try_resplit_at(
        &mut self,
        other: &mut Self,
        new_self_len: usize,
    ) -> Result<(), ResplitError> {
        if self.header_ptr != other.header_ptr {
            Err(ResplitError::DistinctLoaves)
        } else if self.ptr_range.end != other.ptr_range.start {
            Err(ResplitError::PartsAreNotAdjacent)
        } else {
            match self.ptr_range.start.checked_add(new_self_len) {
                Some(middle) if middle <= other.ptr_range.end => {
                    self.ptr_range.end = middle;
                    other.ptr_range.start = middle;
                    Ok(())
                }
                _ => Err(ResplitError::OutOfBounds),
            }
        }
    }
    pub fn try_split_at(&mut self, new_self_len: usize) -> Result<Self, ()> {
        if self.len() < new_self_len {
            return Err(());
        }
        let end = self.ptr_range.end;
        let middle = self.ptr_range.start + new_self_len;
        self.ptr_range.end = middle;

        let header_ptr = self.header_ptr as *mut LoafHeader;
        let header_ref: &LoafHeader = unsafe {
            // safe! pointer points to initialized LoafHeader value,
            // which is currently not mut accessed elsewhere
            &*header_ptr
        };
        let was = header_ref.arc.fetch_add(1, atomic::Ordering::SeqCst);
        if was > usize::MAX {
            std::process::abort();
        }

        Ok(Self {
            header_ptr: self.header_ptr,
            ptr_range: middle..end,
        })
    }
    pub fn as_slice(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.ptr_range.start as *const u8, self.ptr_range.len())
        }
    }
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(self.ptr_range.start as *mut u8, self.ptr_range.len())
        }
    }
    pub fn get_ptr_range(&self) -> &Range<usize> {
        &self.ptr_range
    }
    pub unsafe fn get_ptr_range_mut(&mut self) -> &mut Range<usize> {
        &mut self.ptr_range
    }
}

// consuming functions
impl LoafPart {
    pub fn with_try_set_relative_range(
        mut self,
        new_relative_range: Range<usize>,
    ) -> Result<Self, Self> {
        match self.try_set_relative_range(new_relative_range) {
            Ok(()) => Ok(self),
            Err(()) => Err(self),
        }
    }
    pub fn with_try_join(mut self, mut other: Self) -> Result<Self, (JoinError, [Self; 2])> {
        match self.try_join(&mut other) {
            Ok(()) => Ok(self),
            Err(join_error) => Err((join_error, [self, other])),
        }
    }
    pub fn with_try_split_at(mut self, new_self_len: usize) -> Result<[Self; 2], Self> {
        match self.try_split_at(new_self_len) {
            Ok(tail) => Ok([self, tail]),
            Err(()) => Err(self),
        }
    }
}

impl Drop for LoafPart {
    fn drop(&mut self) {
        let header_ptr = self.header_ptr as *mut LoafHeader;
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
                alloc::dealloc(self.header_ptr as *mut u8, layout)
            }
        }
    }
}
impl AsRef<[u8]> for LoafPart {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}
impl AsMut<[u8]> for LoafPart {
    fn as_mut(&mut self) -> &mut [u8] {
        self.as_slice_mut()
    }
}
