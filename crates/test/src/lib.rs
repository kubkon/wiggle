use proptest::prelude::*;
use std::cell::UnsafeCell;
use std::collections::BTreeSet;
use wiggle_runtime::GuestMemory;

pub type MemSet = BTreeSet<MemArea>;

#[repr(align(4096))]
pub struct HostMemory {
    buffer: UnsafeCell<[u8; 4096]>,
}
impl HostMemory {
    pub fn new() -> Self {
        HostMemory {
            buffer: UnsafeCell::new([0; 4096]),
        }
    }

    pub fn mem_area_strat(align: u32) -> BoxedStrategy<MemArea> {
        prop::num::u32::ANY
            .prop_filter_map("needs to fit in memory", move |p| {
                let p_aligned = p - (p % align); // Align according to argument
                let ptr = p_aligned % 4096; // Put inside memory
                if ptr + align < 4096 {
                    Some(MemArea { ptr, len: align })
                } else {
                    None
                }
            })
            .boxed()
    }

    /// Takes a sorted list or memareas, and gives a sorted list of memareas covering
    /// the parts of memory not covered by the previous
    pub fn invert(regions: &MemSet) -> MemSet {
        let mut out = MemSet::new();
        let mut start = 0;
        for r in regions.iter() {
            let len = r.ptr - start;
            if len > 0 {
                out.insert(MemArea {
                    ptr: start,
                    len: r.ptr - start,
                });
            }
            start = r.ptr + r.len;
        }
        if start < 4096 {
            out.insert(MemArea {
                ptr: start,
                len: 4096 - start,
            });
        }
        out
    }

    pub fn byte_slice_strat(size: u32, exclude: &MemSet) -> BoxedStrategy<MemArea> {
        let available: Vec<MemArea> = Self::invert(exclude)
            .iter()
            .flat_map(|a| a.inside(size))
            .collect();
        prop::sample::select(available).boxed()
    }
}

unsafe impl GuestMemory for HostMemory {
    fn base(&self) -> (*mut u8, u32) {
        unsafe {
            let ptr = self.buffer.get();
            ((*ptr).as_mut_ptr(), (*ptr).len() as u32)
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemArea {
    pub ptr: u32,
    pub len: u32,
}

impl MemArea {
    // This code is a whole lot like the Region::overlaps func thats at the core of the code under
    // test.
    // So, I implemented this one with std::ops::Range so it is less likely I wrote the same bug in two
    // places.
    pub fn overlapping(&self, b: Self) -> bool {
        // a_range is all elems in A
        let a_range = std::ops::Range {
            start: self.ptr,
            end: self.ptr + self.len, // std::ops::Range is open from the right
        };
        // b_range is all elems in B
        let b_range = std::ops::Range {
            start: b.ptr,
            end: b.ptr + b.len,
        };
        // No element in B is contained in A:
        for b_elem in b_range.clone() {
            if a_range.contains(&b_elem) {
                return true;
            }
        }
        // No element in A is contained in B:
        for a_elem in a_range {
            if b_range.contains(&a_elem) {
                return true;
            }
        }
        return false;
    }
    pub fn non_overlapping_set(areas: &MemSet) -> bool {
        // A is all areas
        for a in areas.iter() {
            let set_of_a: MemSet = vec![a].into_iter().cloned().collect();
            // (A, B) is every pair of areas
            for b in areas.difference(&set_of_a) {
                if a.overlapping(*b) {
                    return false;
                }
            }
        }
        return true;
    }

    /// Enumerate all memareas of size `len` inside a given area
    fn inside(&self, len: u32) -> impl Iterator<Item = MemArea> {
        let end: i64 = len as i64 - self.len as i64;
        let start = self.ptr;
        (0..end).into_iter().map(move |v| MemArea {
            ptr: start + v as u32,
            len,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn hostmemory_is_aligned() {
        let h = HostMemory::new();
        assert_eq!(h.base().0 as usize % 4096, 0);
        let h = Box::new(h);
        assert_eq!(h.base().0 as usize % 4096, 0);
    }

    #[test]
    fn invert() {
        fn invert_equality(input: &[MemArea], expected: &[MemArea]) {
            let input: BTreeSet<MemArea> = input.iter().cloned().collect();
            let inverted: Vec<MemArea> = HostMemory::invert(&input).into_iter().collect();
            assert_eq!(expected, inverted.as_slice());
        }

        invert_equality(&[], &[MemArea { ptr: 0, len: 4096 }]);
        invert_equality(
            &[MemArea { ptr: 0, len: 1 }],
            &[MemArea { ptr: 1, len: 4095 }],
        );

        invert_equality(
            &[MemArea { ptr: 1, len: 1 }],
            &[MemArea { ptr: 0, len: 1 }, MemArea { ptr: 2, len: 4094 }],
        );

        invert_equality(
            &[MemArea { ptr: 1, len: 4095 }],
            &[MemArea { ptr: 0, len: 1 }],
        );

        invert_equality(
            &[MemArea { ptr: 0, len: 1 }, MemArea { ptr: 1, len: 4095 }],
            &[],
        );

        invert_equality(
            &[MemArea { ptr: 1, len: 2 }, MemArea { ptr: 4, len: 1 }],
            &[
                MemArea { ptr: 0, len: 1 },
                MemArea { ptr: 3, len: 1 },
                MemArea { ptr: 5, len: 4091 },
            ],
        );
    }

    proptest! {
        #[test]
        // For some random region of decent size
        fn inside(r in HostMemory::mem_area_strat(123)) {
            let set_of_r: MemSet = vec![r].into_iter().collect();
            // All regions outside of r:
            let exterior = HostMemory::invert(&set_of_r);
            // All regions inside of r:
            let interior = r.inside(22);
            for i in interior {
                // i overlaps with r:
                assert!(r.overlapping(i));
                // i is inside r:
                assert!(r.ptr >= i.ptr);
                assert!(r.ptr + r.len >= i.ptr + i.len);
                // the set of exterior and i is non-overlapping
                let mut all = exterior.clone();
                all.insert(i);
                assert!(MemArea::non_overlapping_set(&all));
            }
        }
    }
}

use std::cell::RefCell;
use wiggle_runtime::GuestError;

pub struct WasiCtx {
    pub guest_errors: RefCell<Vec<GuestError>>,
}

impl WasiCtx {
    pub fn new() -> Self {
        Self {
            guest_errors: RefCell::new(vec![]),
        }
    }
}

// Errno is used as a first return value in the functions above, therefore
// it must implement GuestErrorType with type Context = WasiCtx.
// The context type should let you do logging or debugging or whatever you need
// with these errors. We just push them to vecs.
#[macro_export]
macro_rules! impl_errno {
    ( $errno:ty ) => {
        impl wiggle_runtime::GuestErrorType for $errno {
            type Context = WasiCtx;
            fn success() -> $errno {
                <$errno>::Ok
            }
            fn from_error(e: GuestError, ctx: &WasiCtx) -> $errno {
                eprintln!("GUEST ERROR: {:?}", e);
                ctx.guest_errors.borrow_mut().push(e);
                types::Errno::InvalidArg
            }
        }
    };
}
