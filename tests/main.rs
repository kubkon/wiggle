use proptest::prelude::*;
use std::convert::TryFrom;
use wiggle_runtime::{GuestError, GuestPtrMut, GuestString};
use wiggle_test::{impl_errno, HostMemory, MemArea, WasiCtx};

wiggle_generate::from_witx!({
    witx: ["tests/test.witx"],
    ctx: WasiCtx,
});

impl_errno!(types::Errno);

impl foo::Foo for WasiCtx {
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
