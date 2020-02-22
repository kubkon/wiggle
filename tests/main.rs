use proptest::prelude::*;
use std::convert::TryFrom;
use wiggle_runtime::{GuestArray, GuestError, GuestPtr, GuestPtrMut, GuestString};
use wiggle_test::{impl_errno, HostMemory, MemArea, WasiCtx};

wiggle_generate::from_witx!({
    witx: ["tests/test.witx"],
    ctx: WasiCtx,
});

impl_errno!(types::Errno);

impl foo::Foo for WasiCtx {
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

    fn reduce_excuses(
        &mut self,
        excuses: &types::ConstExcuseArray,
    ) -> Result<types::Excuse, types::Errno> {
        let last = wiggle_runtime::GuestTypeClone::read_from_guest(
            &excuses
                .iter()
                .last()
                .expect("input array is non-empty")
                .expect("valid ptr to ptr"),
        )
        .expect("valid ptr to some Excuse value");
        Ok(*last.as_ref().expect("dereferencing ptr should succeed"))
    }

    fn populate_excuses(&mut self, excuses: &types::ExcuseArray) -> Result<(), types::Errno> {
        for excuse in excuses.iter() {
            let ptr_to_ptr =
                wiggle_runtime::GuestTypeClone::read_from_guest(&excuse.expect("valid ptr to ptr"))
                    .expect("valid ptr to some Excuse value");
            let mut ptr = ptr_to_ptr
                .as_ref_mut()
                .expect("dereferencing mut ptr should succeed");
            *ptr = types::Excuse::Sleeping;
        }
        Ok(())
    }

    fn configure_car(
        &mut self,
        old_config: types::CarConfig,
        other_config_ptr: GuestPtr<types::CarConfig>,
    ) -> Result<types::CarConfig, types::Errno> {
        let other_config = *other_config_ptr.as_ref().map_err(|e| {
            eprintln!("old_config_ptr error: {}", e);
            types::Errno::InvalidArg
        })?;
        Ok(old_config ^ other_config)
    }

    fn hello_string(&mut self, a_string: &GuestString<'_>) -> Result<u32, types::Errno> {
        let as_ref = a_string.as_ref().expect("deref ptr should succeed");
        let as_str = as_ref.as_str().expect("valid UTF-8 string");
        println!("a_string='{}'", as_str);
        Ok(as_str.len() as u32)
    }

    fn cookie_cutter(&mut self, init_cookie: types::Cookie) -> Result<types::Bool, types::Errno> {
        let res = if init_cookie == types::Cookie::START {
            types::Bool::True
        } else {
            types::Bool::False
        };
        Ok(res)
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
        let sum_err = foo::sum_of_pair(
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

        let res = foo::sum_of_pair_of_ptrs(
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
struct ReduceExcusesExcercise {
    excuse_values: Vec<types::Excuse>,
    excuse_ptr_locs: Vec<MemArea>,
    array_ptr_loc: MemArea,
    array_len_loc: MemArea,
    return_ptr_loc: MemArea,
}

impl ReduceExcusesExcercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (1..256u32)
            .prop_flat_map(|len| {
                let len_usize = len as usize;
                (
                    proptest::collection::vec(excuse_strat(), len_usize..=len_usize),
                    proptest::collection::vec(HostMemory::mem_area_strat(4), len_usize..=len_usize),
                    HostMemory::mem_area_strat(4 * len),
                    HostMemory::mem_area_strat(4),
                    HostMemory::mem_area_strat(4),
                )
            })
            .prop_map(
                |(excuse_values, excuse_ptr_locs, array_ptr_loc, array_len_loc, return_ptr_loc)| {
                    Self {
                        excuse_values,
                        excuse_ptr_locs,
                        array_ptr_loc,
                        array_len_loc,
                        return_ptr_loc,
                    }
                },
            )
            .prop_filter("non-overlapping pointers", |e| {
                let mut all = vec![&e.array_ptr_loc, &e.array_len_loc, &e.return_ptr_loc];
                all.extend(e.excuse_ptr_locs.iter());
                MemArea::non_overlapping_set(&all)
            })
            .boxed()
    }

    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut guest_memory = host_memory.guest_memory();

        // Populate memory with pointers to generated Excuse values
        for (&excuse, ptr) in self.excuse_values.iter().zip(self.excuse_ptr_locs.iter()) {
            *guest_memory
                .ptr_mut(ptr.ptr)
                .expect("ptr mut to Excuse value")
                .as_ref_mut()
                .expect("deref ptr mut to Excuse value") = excuse;
        }

        // Populate array length info
        *guest_memory
            .ptr_mut(self.array_len_loc.ptr)
            .expect("ptr to array len")
            .as_ref_mut()
            .expect("deref ptr mut to array len") = self.excuse_ptr_locs.len() as u32;

        // Populate the array with pointers to generated Excuse values
        {
            let mut next: GuestPtrMut<'_, GuestPtr<types::Excuse>> = guest_memory
                .ptr_mut(self.array_ptr_loc.ptr)
                .expect("ptr to array mut");
            for ptr in &self.excuse_ptr_locs {
                next.write_ptr_to_guest(
                    &guest_memory
                        .ptr::<types::Excuse>(ptr.ptr)
                        .expect("ptr to Excuse value"),
                );
                next = next.elem(1).expect("increment ptr by 1");
            }
        }

        let res = foo::reduce_excuses(
            &mut ctx,
            &mut guest_memory,
            self.array_ptr_loc.ptr as i32,
            self.array_len_loc.ptr as i32,
            self.return_ptr_loc.ptr as i32,
        );

        assert_eq!(res, types::Errno::Ok.into(), "reduce excuses errno");

        let expected = *self
            .excuse_values
            .last()
            .expect("generated vec of excuses should be non-empty");
        let given: types::Excuse = *guest_memory
            .ptr(self.return_ptr_loc.ptr)
            .expect("ptr to returned value")
            .as_ref()
            .expect("deref ptr to returned value");
        assert_eq!(expected, given, "reduce excuses return val");
    }
}
proptest! {
    #[test]
    fn reduce_excuses(e in ReduceExcusesExcercise::strat()) {
        e.test()
    }
}

fn excuse_strat() -> impl Strategy<Value = types::Excuse> {
    prop_oneof![
        Just(types::Excuse::DogAte),
        Just(types::Excuse::Traffic),
        Just(types::Excuse::Sleeping),
    ]
    .boxed()
}

#[derive(Debug)]
struct PopulateExcusesExcercise {
    array_ptr_loc: MemArea,
    array_len_loc: MemArea,
    elements: Vec<MemArea>,
}

impl PopulateExcusesExcercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (1..256u32)
            .prop_flat_map(|len| {
                let len_usize = len as usize;
                (
                    HostMemory::mem_area_strat(4 * len),
                    HostMemory::mem_area_strat(4),
                    proptest::collection::vec(HostMemory::mem_area_strat(4), len_usize..=len_usize),
                )
            })
            .prop_map(|(array_ptr_loc, array_len_loc, elements)| Self {
                array_ptr_loc,
                array_len_loc,
                elements,
            })
            .prop_filter("non-overlapping pointers", |e| {
                let mut all = vec![&e.array_ptr_loc, &e.array_len_loc];
                all.extend(e.elements.iter());
                MemArea::non_overlapping_set(&all)
            })
            .boxed()
    }

    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut guest_memory = host_memory.guest_memory();

        // Populate array length info
        *guest_memory
            .ptr_mut(self.array_len_loc.ptr)
            .expect("ptr mut to array len")
            .as_ref_mut()
            .expect("deref ptr mut to array len") = self.elements.len() as u32;

        // Populate array with valid pointers to Excuse type in memory
        {
            let mut next: GuestPtrMut<'_, GuestPtrMut<types::Excuse>> = guest_memory
                .ptr_mut(self.array_ptr_loc.ptr)
                .expect("ptr mut to the first element of array");
            for ptr in &self.elements {
                next.write_ptr_to_guest(
                    &guest_memory
                        .ptr_mut::<types::Excuse>(ptr.ptr)
                        .expect("ptr mut to Excuse value"),
                );
                next = next.elem(1).expect("increment ptr by 1");
            }
        }

        let res = foo::populate_excuses(
            &mut ctx,
            &mut guest_memory,
            self.array_ptr_loc.ptr as i32,
            self.array_len_loc.ptr as i32,
        );
        assert_eq!(res, types::Errno::Ok.into(), "populate excuses errno");

        let arr: GuestArray<'_, GuestPtr<'_, types::Excuse>> = guest_memory
            .ptr(self.array_ptr_loc.ptr)
            .expect("ptr to the first element of array")
            .array(self.elements.len() as u32)
            .expect("as array");
        for el in arr.iter() {
            let ptr_to_ptr =
                wiggle_runtime::GuestTypeClone::read_from_guest(&el.expect("valid ptr to ptr"))
                    .expect("valid ptr to some Excuse value");
            assert_eq!(
                *ptr_to_ptr
                    .as_ref()
                    .expect("dereferencing ptr to some Excuse value"),
                types::Excuse::Sleeping,
                "element should equal Excuse::Sleeping"
            );
        }
    }
}
proptest! {
    #[test]
    fn populate_excuses(e in PopulateExcusesExcercise::strat()) {
        e.test()
    }
}

fn car_config_strat() -> impl Strategy<Value = types::CarConfig> {
    (1u8..=types::CarConfig::ALL_FLAGS.into())
        .prop_map(|v| {
            types::CarConfig::try_from(v).expect("invalid value for types::CarConfig flag")
        })
        .boxed()
}

#[derive(Debug)]
struct ConfigureCarExercise {
    old_config: types::CarConfig,
    other_config: types::CarConfig,
    other_config_by_ptr: MemArea,
    return_ptr_loc: MemArea,
}

impl ConfigureCarExercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (
            car_config_strat(),
            car_config_strat(),
            HostMemory::mem_area_strat(4),
            HostMemory::mem_area_strat(4),
        )
            .prop_map(
                |(old_config, other_config, other_config_by_ptr, return_ptr_loc)| Self {
                    old_config,
                    other_config,
                    other_config_by_ptr,
                    return_ptr_loc,
                },
            )
            .prop_filter("non-overlapping ptrs", |e| {
                MemArea::non_overlapping_set(&[&e.other_config_by_ptr, &e.return_ptr_loc])
            })
            .boxed()
    }

    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut guest_memory = host_memory.guest_memory();

        // Populate input ptr
        *guest_memory
            .ptr_mut(self.other_config_by_ptr.ptr)
            .expect("ptr mut to CarConfig")
            .as_ref_mut()
            .expect("deref ptr mut to CarConfig") = self.other_config;

        let res = foo::configure_car(
            &mut ctx,
            &mut guest_memory,
            self.old_config.into(),
            self.other_config_by_ptr.ptr as i32,
            self.return_ptr_loc.ptr as i32,
        );
        assert_eq!(res, types::Errno::Ok.into(), "configure car errno");

        let res_config = *guest_memory
            .ptr::<types::CarConfig>(self.return_ptr_loc.ptr)
            .expect("ptr to returned CarConfig")
            .as_ref()
            .expect("deref to CarConfig value");

        assert_eq!(
            self.old_config ^ self.other_config,
            res_config,
            "returned CarConfig should be an XOR of inputs"
        );
    }
}
proptest! {
    #[test]
    fn configure_car(e in ConfigureCarExercise::strat()) {
        e.test()
    }
}

fn test_string_strategy() -> impl Strategy<Value = String> {
    "\\p{Greek}{1,256}"
}

#[derive(Debug)]
struct HelloStringExercise {
    test_word: String,
    string_ptr_loc: MemArea,
    string_len_loc: MemArea,
    return_ptr_loc: MemArea,
}

impl HelloStringExercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (test_string_strategy(),)
            .prop_flat_map(|(test_word,)| {
                (
                    Just(test_word.clone()),
                    HostMemory::mem_area_strat(test_word.len() as u32),
                    HostMemory::mem_area_strat(4),
                    HostMemory::mem_area_strat(4),
                )
            })
            .prop_map(
                |(test_word, string_ptr_loc, string_len_loc, return_ptr_loc)| Self {
                    test_word,
                    string_ptr_loc,
                    string_len_loc,
                    return_ptr_loc,
                },
            )
            .prop_filter("non-overlapping pointers", |e| {
                MemArea::non_overlapping_set(&[
                    &e.string_ptr_loc,
                    &e.string_len_loc,
                    &e.return_ptr_loc,
                ])
            })
            .boxed()
    }

    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut guest_memory = host_memory.guest_memory();

        // Populate string length
        *guest_memory
            .ptr_mut(self.string_len_loc.ptr)
            .expect("ptr mut to string len")
            .as_ref_mut()
            .expect("deref ptr mut to string len") = self.test_word.len() as u32;

        // Populate string in guest's memory
        {
            let mut next: GuestPtrMut<'_, u8> = guest_memory
                .ptr_mut(self.string_ptr_loc.ptr)
                .expect("ptr mut to the first byte of string");
            for byte in self.test_word.as_bytes() {
                *next.as_ref_mut().expect("deref mut") = *byte;
                next = next.elem(1).expect("increment ptr by 1");
            }
        }

        let res = foo::hello_string(
            &mut ctx,
            &mut guest_memory,
            self.string_ptr_loc.ptr as i32,
            self.string_len_loc.ptr as i32,
            self.return_ptr_loc.ptr as i32,
        );
        assert_eq!(res, types::Errno::Ok.into(), "hello string errno");

        let given = *guest_memory
            .ptr::<u32>(self.return_ptr_loc.ptr)
            .expect("ptr to return value")
            .as_ref()
            .expect("deref ptr to return value");
        assert_eq!(self.test_word.len() as u32, given);
    }
}
proptest! {
    #[test]
    fn hello_string(e in HelloStringExercise::strat()) {
        e.test()
    }
}

fn cookie_strat() -> impl Strategy<Value = types::Cookie> {
    (0..std::u64::MAX)
        .prop_map(|x| types::Cookie::try_from(x).expect("within range of cookie"))
        .boxed()
}

#[derive(Debug)]
struct CookieCutterExercise {
    cookie: types::Cookie,
    return_ptr_loc: MemArea,
}

impl CookieCutterExercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (cookie_strat(), HostMemory::mem_area_strat(4))
            .prop_map(|(cookie, return_ptr_loc)| Self {
                cookie,
                return_ptr_loc,
            })
            .boxed()
    }

    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut guest_memory = host_memory.guest_memory();

        let res = foo::cookie_cutter(
            &mut ctx,
            &mut guest_memory,
            self.cookie.into(),
            self.return_ptr_loc.ptr as i32,
        );
        assert_eq!(res, types::Errno::Ok.into(), "cookie cutter errno");

        let is_cookie_start = *guest_memory
            .ptr::<types::Bool>(self.return_ptr_loc.ptr)
            .expect("ptr to returned Bool")
            .as_ref()
            .expect("deref to Bool value");

        assert_eq!(
            if is_cookie_start == types::Bool::True {
                true
            } else {
                false
            },
            self.cookie == types::Cookie::START,
            "returned Bool should test if input was Cookie::START",
        );
    }
}
proptest! {
    #[test]
    fn cookie_cutter(e in CookieCutterExercise::strat()) {
        e.test()
    }
}
