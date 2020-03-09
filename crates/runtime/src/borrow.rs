use crate::region::Region;
use crate::{GuestError, GuestPtr, GuestType};

#[derive(Debug)]
pub struct GuestBorrows {
    borrows: Vec<Region>,
}

impl GuestBorrows {
    pub fn new() -> Self {
        Self {
            borrows: Vec::new(),
        }
    }

    fn is_borrowed(&self, r: Region) -> bool {
        !self.borrows.iter().all(|b| !b.overlaps(r))
    }

    pub(crate) fn borrow(&mut self, r: Region) -> Result<(), GuestError> {
        if self.is_borrowed(r) {
            Err(GuestError::PtrBorrowed(r))
        } else {
            self.borrows.push(r);
            Ok(())
        }
    }

    /// Borrow the region of memory pointed to by a `GuestPtr`. This is required for safety if
    /// you are dereferencing `GuestPtr`s while holding a reference to a slice via
    /// `GuestPtr::as_raw`.
    pub fn borrow_pointee<'a, T>(&mut self, p: &GuestPtr<'a, T>) -> Result<(), GuestError>
    where
        T: GuestType<'a>,
    {
        self.borrow(Region {
            start: p.offset(),
            len: T::guest_size(),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn nonoverlapping() {
        let mut bs = GuestBorrows::new();
        let r1 = Region::new(0, 10);
        let r2 = Region::new(10, 10);
        assert!(!r1.overlaps(r2));
        bs.borrow(r1).expect("can borrow r1");
        bs.borrow(r2).expect("can borrow r2");

        let mut bs = GuestBorrows::new();
        let r1 = Region::new(10, 10);
        let r2 = Region::new(0, 10);
        assert!(!r1.overlaps(r2));
        bs.borrow(r1).expect("can borrow r1");
        bs.borrow(r2).expect("can borrow r2");
    }

    #[test]
    fn overlapping() {
        let mut bs = GuestBorrows::new();
        let r1 = Region::new(0, 10);
        let r2 = Region::new(9, 10);
        assert!(r1.overlaps(r2));
        bs.borrow(r1).expect("can borrow r1");
        assert!(bs.borrow(r2).is_err(), "cant borrow r2");

        let mut bs = GuestBorrows::new();
        let r1 = Region::new(0, 10);
        let r2 = Region::new(2, 5);
        assert!(r1.overlaps(r2));
        bs.borrow(r1).expect("can borrow r1");
        assert!(bs.borrow(r2).is_err(), "cant borrow r2");

        let mut bs = GuestBorrows::new();
        let r1 = Region::new(9, 10);
        let r2 = Region::new(0, 10);
        assert!(r1.overlaps(r2));
        bs.borrow(r1).expect("can borrow r1");
        assert!(bs.borrow(r2).is_err(), "cant borrow r2");

        let mut bs = GuestBorrows::new();
        let r1 = Region::new(2, 5);
        let r2 = Region::new(0, 10);
        assert!(r1.overlaps(r2));
        bs.borrow(r1).expect("can borrow r1");
        assert!(bs.borrow(r2).is_err(), "cant borrow r2");

        let mut bs = GuestBorrows::new();
        let r1 = Region::new(2, 5);
        let r2 = Region::new(10, 5);
        let r3 = Region::new(15, 5);
        let r4 = Region::new(0, 10);
        assert!(r1.overlaps(r4));
        bs.borrow(r1).expect("can borrow r1");
        bs.borrow(r2).expect("can borrow r2");
        bs.borrow(r3).expect("can borrow r3");
        assert!(bs.borrow(r4).is_err(), "cant borrow r4");
    }
}
