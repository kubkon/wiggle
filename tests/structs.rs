use proptest::prelude::*;
use wiggle_runtime::{GuestError, GuestPtr};
use wiggle_test::{impl_errno, HostMemory, MemArea, WasiCtx};

wiggle_generate::from_witx!({
    witx: ["tests/structs.witx"],
    ctx: WasiCtx,
});

impl_errno!(types::Errno);

impl structs::Structs for WasiCtx {
    fn sum_of_pair(&mut self, an_pair: &types::PairInts) -> Result<i64, types::Errno> {
        Ok(an_pair.first as i64 + an_pair.second as i64)
    }

    fn sum_of_pair_of_ptrs(&mut self, an_pair: &types::PairIntPtrs) -> Result<i64, types::Errno> {
        let first = *an_pair
            .first
            .as_ref()
            .expect("dereferencing GuestPtr should succeed");
        let second = *an_pair
            .second
            .as_ref()
            .expect("dereferncing GuestPtr should succeed");
        Ok(first as i64 + second as i64)
    }

    fn sum_of_int_and_ptr(&mut self, an_pair: &types::PairIntAndPtr) -> Result<i64, types::Errno> {
        let first = *an_pair
            .first
            .as_ref()
            .expect("dereferencing GuestPtr should succeed");
        let second = an_pair.second as i64;
        Ok(first as i64 + second)
    }

    fn return_pair_ints(&mut self) -> Result<types::PairInts, types::Errno> {
        Ok(types::PairInts {
            first: 10,
            second: 20,
        })
    }

    fn return_pair_of_ptrs<'a>(
        &mut self,
        first: GuestPtr<'a, i32>,
        second: GuestPtr<'a, i32>,
    ) -> Result<types::PairIntPtrs<'a>, types::Errno> {
        Ok(types::PairIntPtrs { first, second })
    }
}

#[derive(Debug)]
struct SumOfPairExercise {
    pub input: types::PairInts,
    pub input_loc: MemArea,
    pub return_loc: MemArea,
}

impl SumOfPairExercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (
            prop::num::i32::ANY,
            prop::num::i32::ANY,
            HostMemory::mem_area_strat(8),
            HostMemory::mem_area_strat(8),
        )
            .prop_map(|(first, second, input_loc, return_loc)| SumOfPairExercise {
                input: types::PairInts { first, second },
                input_loc,
                return_loc,
            })
            .prop_filter("non-overlapping pointers", |e| {
                MemArea::non_overlapping_set(&[&e.input_loc, &e.return_loc])
            })
            .boxed()
    }

    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut guest_memory = host_memory.guest_memory();

        *guest_memory
            .ptr_mut(self.input_loc.ptr)
            .expect("input ptr")
            .as_ref_mut()
            .expect("input ref_mut") = self.input.first;
        *guest_memory
            .ptr_mut(self.input_loc.ptr + 4)
            .expect("input ptr")
            .as_ref_mut()
            .expect("input ref_mut") = self.input.second;
        let sum_err = structs::sum_of_pair(
            &mut ctx,
            &mut guest_memory,
            self.input_loc.ptr as i32,
            self.return_loc.ptr as i32,
        );

        assert_eq!(sum_err, types::Errno::Ok.into(), "sum errno");

        let return_val: i64 = *guest_memory
            .ptr(self.return_loc.ptr)
            .expect("return ptr")
            .as_ref()
            .expect("return ref");

        assert_eq!(
            return_val,
            self.input.first as i64 + self.input.second as i64,
            "sum return value"
        );
    }
}

proptest! {
    #[test]
    fn sum_of_pair(e in SumOfPairExercise::strat()) {
        e.test();
    }
}

#[derive(Debug)]
struct SumPairPtrsExercise {
    input_first: i32,
    input_second: i32,
    input_first_loc: MemArea,
    input_second_loc: MemArea,
    input_struct_loc: MemArea,
    return_loc: MemArea,
}

impl SumPairPtrsExercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (
            prop::num::i32::ANY,
            prop::num::i32::ANY,
            HostMemory::mem_area_strat(4),
            HostMemory::mem_area_strat(4),
            HostMemory::mem_area_strat(8),
            HostMemory::mem_area_strat(8),
        )
            .prop_map(
                |(
                    input_first,
                    input_second,
                    input_first_loc,
                    input_second_loc,
                    input_struct_loc,
                    return_loc,
                )| SumPairPtrsExercise {
                    input_first,
                    input_second,
                    input_first_loc,
                    input_second_loc,
                    input_struct_loc,
                    return_loc,
                },
            )
            .prop_filter("non-overlapping pointers", |e| {
                MemArea::non_overlapping_set(&[
                    &e.input_first_loc,
                    &e.input_second_loc,
                    &e.input_struct_loc,
                    &e.return_loc,
                ])
            })
            .boxed()
    }
    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut guest_memory = host_memory.guest_memory();

        *guest_memory
            .ptr_mut(self.input_first_loc.ptr)
            .expect("input_first ptr")
            .as_ref_mut()
            .expect("input_first ref") = self.input_first;
        *guest_memory
            .ptr_mut(self.input_second_loc.ptr)
            .expect("input_second ptr")
            .as_ref_mut()
            .expect("input_second ref") = self.input_second;

        *guest_memory
            .ptr_mut(self.input_struct_loc.ptr)
            .expect("input_struct ptr")
            .as_ref_mut()
            .expect("input_struct ref") = self.input_first_loc.ptr;
        *guest_memory
            .ptr_mut(self.input_struct_loc.ptr + 4)
            .expect("input_struct ptr")
            .as_ref_mut()
            .expect("input_struct ref") = self.input_second_loc.ptr;

        let res = structs::sum_of_pair_of_ptrs(
            &mut ctx,
            &mut guest_memory,
            self.input_struct_loc.ptr as i32,
            self.return_loc.ptr as i32,
        );

        assert_eq!(res, types::Errno::Ok.into(), "sum of pair of ptrs errno");

        let doubled: i64 = *guest_memory
            .ptr(self.return_loc.ptr)
            .expect("return ptr")
            .as_ref()
            .expect("return ref");

        assert_eq!(
            doubled,
            (self.input_first as i64) + (self.input_second as i64),
            "sum of pair of ptrs return val"
        );
    }
}
proptest! {
    #[test]
    fn sum_of_pair_of_ptrs(e in SumPairPtrsExercise::strat()) {
        e.test()
    }
}

#[derive(Debug)]
struct SumIntAndPtrExercise {
    input_first: i32,
    input_second: i32,
    input_first_loc: MemArea,
    input_struct_loc: MemArea,
    return_loc: MemArea,
}

impl SumIntAndPtrExercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (
            prop::num::i32::ANY,
            prop::num::i32::ANY,
            HostMemory::mem_area_strat(4),
            HostMemory::mem_area_strat(8),
            HostMemory::mem_area_strat(8),
        )
            .prop_map(
                |(input_first, input_second, input_first_loc, input_struct_loc, return_loc)| {
                    SumIntAndPtrExercise {
                        input_first,
                        input_second,
                        input_first_loc,
                        input_struct_loc,
                        return_loc,
                    }
                },
            )
            .prop_filter("non-overlapping pointers", |e| {
                MemArea::non_overlapping_set(&[
                    &e.input_first_loc,
                    &e.input_struct_loc,
                    &e.return_loc,
                ])
            })
            .boxed()
    }
    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut guest_memory = host_memory.guest_memory();

        *guest_memory
            .ptr_mut(self.input_first_loc.ptr)
            .expect("input_first ptr")
            .as_ref_mut()
            .expect("input_first ref") = self.input_first;
        *guest_memory
            .ptr_mut(self.input_struct_loc.ptr)
            .expect("input_struct ptr")
            .as_ref_mut()
            .expect("input_struct ref") = self.input_first_loc.ptr;
        *guest_memory
            .ptr_mut(self.input_struct_loc.ptr + 4)
            .expect("input_struct ptr")
            .as_ref_mut()
            .expect("input_struct ref") = self.input_second;

        let res = structs::sum_of_int_and_ptr(
            &mut ctx,
            &mut guest_memory,
            self.input_struct_loc.ptr as i32,
            self.return_loc.ptr as i32,
        );

        assert_eq!(res, types::Errno::Ok.into(), "sum of int and ptr errno");

        let doubled: i64 = *guest_memory
            .ptr(self.return_loc.ptr)
            .expect("return ptr")
            .as_ref()
            .expect("return ref");

        assert_eq!(
            doubled,
            (self.input_first as i64) + (self.input_second as i64),
            "sum of pair of ptrs return val"
        );
    }
}
proptest! {
    #[test]
    fn sum_of_int_and_ptr(e in SumIntAndPtrExercise::strat()) {
        e.test()
    }
}

#[derive(Debug)]
struct ReturnPairInts {
    pub return_loc: MemArea,
}

impl ReturnPairInts {
    pub fn strat() -> BoxedStrategy<Self> {
        HostMemory::mem_area_strat(8)
            .prop_map(|return_loc| ReturnPairInts { return_loc })
            .boxed()
    }

    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut guest_memory = host_memory.guest_memory();

        let err =
            structs::return_pair_ints(&mut ctx, &mut guest_memory, self.return_loc.ptr as i32);

        assert_eq!(err, types::Errno::Ok.into(), "return struct errno");

        let return_struct: types::PairInts = *guest_memory
            .ptr(self.return_loc.ptr)
            .expect("return ptr")
            .as_ref()
            .expect("return ref");

        assert_eq!(
            return_struct,
            types::PairInts {
                first: 10,
                second: 20
            },
            "return_pair_ints return value"
        );
    }
}

proptest! {
    #[test]
    fn return_pair_ints(e in ReturnPairInts::strat()) {
        e.test();
    }
}

#[derive(Debug)]
struct ReturnPairPtrsExercise {
    input_first: i32,
    input_second: i32,
    input_first_loc: MemArea,
    input_second_loc: MemArea,
    return_loc: MemArea,
}

impl ReturnPairPtrsExercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (
            prop::num::i32::ANY,
            prop::num::i32::ANY,
            HostMemory::mem_area_strat(4),
            HostMemory::mem_area_strat(4),
            HostMemory::mem_area_strat(8),
        )
            .prop_map(
                |(input_first, input_second, input_first_loc, input_second_loc, return_loc)| {
                    ReturnPairPtrsExercise {
                        input_first,
                        input_second,
                        input_first_loc,
                        input_second_loc,
                        return_loc,
                    }
                },
            )
            .prop_filter("non-overlapping pointers", |e| {
                MemArea::non_overlapping_set(&[
                    &e.input_first_loc,
                    &e.input_second_loc,
                    &e.return_loc,
                ])
            })
            .boxed()
    }
    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut guest_memory = host_memory.guest_memory();

        *guest_memory
            .ptr_mut(self.input_first_loc.ptr)
            .expect("input_first ptr")
            .as_ref_mut()
            .expect("input_first ref") = self.input_first;
        *guest_memory
            .ptr_mut(self.input_second_loc.ptr)
            .expect("input_second ptr")
            .as_ref_mut()
            .expect("input_second ref") = self.input_second;

        let res = structs::return_pair_of_ptrs(
            &mut ctx,
            &mut guest_memory,
            self.input_first_loc.ptr as i32,
            self.input_second_loc.ptr as i32,
            self.return_loc.ptr as i32,
        );

        assert_eq!(res, types::Errno::Ok.into(), "return pair of ptrs errno");

        let ptr_pair_int_ptrs: GuestPtr<types::PairIntPtrs<'_>> =
            guest_memory.ptr(self.return_loc.ptr).expect("return ptr");
        let ret_first_ptr: GuestPtr<i32> = ptr_pair_int_ptrs
            .cast::<GuestPtr<i32>>(0u32)
            .expect("extract ptr to first element in struct")
            .read()
            .expect("read ptr to first element in struct");
        let ret_second_ptr: GuestPtr<i32> = ptr_pair_int_ptrs
            .cast::<GuestPtr<i32>>(4u32)
            .expect("extract ptr to second element in struct")
            .read()
            .expect("read ptr to second element in struct");
        assert_eq!(
            self.input_first,
            *ret_first_ptr
                .as_ref()
                .expect("deref extracted ptr to first element")
        );
        assert_eq!(
            self.input_second,
            *ret_second_ptr
                .as_ref()
                .expect("deref extracted ptr to second element")
        );
    }
}
proptest! {
    #[test]
    fn return_pair_of_ptrs(e in ReturnPairPtrsExercise::strat()) {
        e.test()
    }
}
