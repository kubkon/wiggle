// FIXME: parameterize macro on what ctx type is used here
generate::from_witx!({
    witx: ["tests/test.witx"],
    ctx: WasiCtx,
});

use crate::foo::Foo;

pub struct WasiCtx {
    guest_errors: Vec<::memory::GuestError>,
}

impl WasiCtx {
    pub fn new() -> Self {
        Self {
            guest_errors: vec![],
        }
    }
}

impl foo::Foo for WasiCtx {
    fn bar(&mut self, an_int: u32, an_float: f32) -> Result<(), types::Errno> {
        println!("BAR: {} {}", an_int, an_float);
        Ok(())
    }

    fn baz(
        &mut self,
        excuse: types::Excuse,
        a_better_excuse_by_reference: ::memory::GuestPtrMut<types::Excuse>,
        a_lamer_excuse_by_reference: ::memory::GuestPtr<types::Excuse>,
        two_layers_of_excuses: ::memory::GuestPtrMut<::memory::GuestPtr<types::Excuse>>,
    ) -> Result<(), types::Errno> {
        // Read enum value from mutable:
        let mut a_better_excuse_ref: ::memory::GuestRefMut<types::Excuse> =
            a_better_excuse_by_reference.as_ref_mut().map_err(|e| {
                eprintln!("a_better_excuse_by_reference error: {}", e);
                types::Errno::InvalidArg
            })?;
        let a_better_excuse: types::Excuse = *a_better_excuse_ref;

        // Read enum value from immutable ptr:
        let a_lamer_excuse = *a_lamer_excuse_by_reference.as_ref().map_err(|e| {
            eprintln!("a_lamer_excuse_by_reference error: {}", e);
            types::Errno::InvalidArg
        })?;

        // Write enum to mutable ptr:
        *a_better_excuse_ref = a_lamer_excuse;

        // Read ptr value from mutable ptr:
        let one_layer_down: ::memory::GuestPtr<types::Excuse> =
            two_layers_of_excuses.read_ptr_from_guest().map_err(|e| {
                eprintln!("one_layer_down error: {}", e);
                types::Errno::InvalidArg
            })?;

        // Read enum value from that ptr:
        let two_layers_down: types::Excuse = *one_layer_down.as_ref().map_err(|e| {
            eprintln!("two_layers_down error: {}", e);
            types::Errno::InvalidArg
        })?;

        // Write ptr value to mutable ptr:
        two_layers_of_excuses.write_ptr_to_guest(&a_better_excuse_by_reference.as_immut());

        println!(
            "BAZ: excuse: {:?}, better excuse: {:?}, lamer excuse: {:?}, two layers down: {:?}",
            excuse, a_better_excuse, a_lamer_excuse, two_layers_down
        );
        Ok(())
    }

    fn bat(&mut self, an_int: u32) -> Result<f32, types::Errno> {
        Ok((an_int as f32) * 2.0)
    }

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
}
// Errno is used as a first return value in the functions above, therefore
// it must implement GuestErrorType with type Context = WasiCtx.
// The context type should let you do logging or debugging or whatever you need
// with these errors. We just push them to vecs.
impl ::memory::GuestErrorType for types::Errno {
    type Context = WasiCtx;
    fn success() -> types::Errno {
        types::Errno::Ok
    }
    fn from_error(e: ::memory::GuestError, ctx: &mut WasiCtx) -> types::Errno {
        ctx.guest_errors.push(e);
        types::Errno::InvalidArg
    }
}

#[test]
fn bat() {
    let mut ctx = WasiCtx::new();
    assert_eq!(ctx.bat(2), Ok(4.0));
}

#[test]
fn sum_of_pair() {
    let mut ctx = WasiCtx::new();
    let pair = types::PairInts {
        first: 1,
        second: 2,
    };
    assert_eq!(ctx.sum_of_pair(&pair), Ok(3));
}

#[test]
fn sum_of_pair_of_ptrs() {
    let mut ctx = WasiCtx::new();
    let host_memory = &mut [0u8; 4096];
    let guest_memory = memory::GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);
    {
        let first_mut: memory::GuestPtrMut<i32> = guest_memory.ptr_mut(0).unwrap();
        let mut x = first_mut.as_ref_mut().unwrap();
        *x = 1;
        let second_mut: memory::GuestPtrMut<i32> = guest_memory.ptr_mut(4).unwrap();
        let mut x = second_mut.as_ref_mut().unwrap();
        *x = 2;
    }
    let first: memory::GuestPtr<i32> = guest_memory
        .ptr(0)
        .expect("GuestPtr<i32> fits in the memory");
    let second: memory::GuestPtr<i32> = guest_memory
        .ptr(4)
        .expect("GuestPtr<i32> fits in the memory");
    let pair = types::PairIntPtrs { first, second };
    assert_eq!(ctx.sum_of_pair_of_ptrs(&pair), Ok(3));
}

