#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Region {
    pub start: u32,
    pub len: u32,
}

impl Region {
    pub fn new(start: u32, len: u32) -> Self {
        Self { start, len }
    }

    pub fn overlaps(&self, rhs: Region) -> bool {
        let self_start = self.start as u64;
        let self_end = self_start + (self.len - 1) as u64;

        let rhs_start = rhs.start as u64;
        let rhs_end = rhs_start + (rhs.len - 1) as u64;

        if self_start <= rhs_start {
            self_end >= rhs_start
        } else {
            rhs_end >= self_start
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn nonoverlapping() {
        let r1 = Region::new(0, 10);
        let r2 = Region::new(10, 10);
        assert!(!r1.overlaps(r2));

        let r1 = Region::new(10, 10);
        let r2 = Region::new(0, 10);
        assert!(!r1.overlaps(r2));
    }

    #[test]
    fn overlapping() {
        let r1 = Region::new(0, 10);
        let r2 = Region::new(9, 10);
        assert!(r1.overlaps(r2));

        let r1 = Region::new(0, 10);
        let r2 = Region::new(2, 5);
        assert!(r1.overlaps(r2));

        let r1 = Region::new(9, 10);
        let r2 = Region::new(0, 10);
        assert!(r1.overlaps(r2));

        let r1 = Region::new(2, 5);
        let r2 = Region::new(0, 10);
        assert!(r1.overlaps(r2));
    }
}
