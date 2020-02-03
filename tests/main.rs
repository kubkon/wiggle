use memory::GuestRef;

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
        println!("BAZ excuse {:?}", excuse);
        // Read enum value from mutable:
        let mut a_better_excuse_ref: ::memory::GuestRefMut<types::Excuse> =
            a_better_excuse_by_reference.as_ref_mut().map_err(|e| {
                eprintln!("a_better_excuse_by_reference error: {}", e);
                types::Errno::InvalidArg
            })?;
        let a_better_excuse: types::Excuse = *a_better_excuse_ref;
        println!("a_better_excuse {:?}", a_better_excuse);

        // Read enum value from immutable ptr:
        let a_lamer_excuse = *a_lamer_excuse_by_reference.as_ref().map_err(|e| {
            eprintln!("a_lamer_excuse_by_reference error: {}", e);
            types::Errno::InvalidArg
        })?;
        println!("a_lamer_excuse {:?}", a_lamer_excuse);

        // Write enum to mutable ptr:
        *a_better_excuse_ref = a_lamer_excuse;
        println!("wrote to a_better_excuse_ref {:?}", *a_better_excuse_ref);

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
        println!("two layers down {:?}", two_layers_down);

        // Write ptr value to mutable ptr:
        two_layers_of_excuses.write_ptr_to_guest(&a_better_excuse_by_reference.as_immut());

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

#[repr(align(4096))]
struct HostMemory {
    buffer: [u8; 4096],
}
impl HostMemory {
    pub fn new() -> Self {
        HostMemory { buffer: [0; 4096] }
    }
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.buffer.as_mut_ptr()
    }
    pub fn len(&self) -> usize {
        self.buffer.len()
    }
}

#[test]
fn hostmemory_is_aligned() {
    let mut h = HostMemory::new();
    assert_eq!(h.as_mut_ptr() as usize % 4096, 0);
    let mut h = Box::new(HostMemory::new());
    assert_eq!(h.as_mut_ptr() as usize % 4096, 0);
}

#[test]
fn bat() {
    let mut ctx = WasiCtx::new();
    let mut host_memory = HostMemory::new();
    let mut guest_memory =
        memory::GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);

    let input = 2;
    let return_loc = 0;

    let bat_err = foo::bat(&mut ctx, &mut guest_memory, input, return_loc);

    let return_val: GuestRef<f32> = guest_memory
        .ptr(return_loc as u32)
        .expect("return loc ptr")
        .as_ref()
        .expect("return val ref");
    assert_eq!(bat_err, types::Errno::Ok.into());
    assert_eq!(*return_val, (input as f32) * 2.0);
}

#[test]
fn baz() {
    let mut ctx = WasiCtx::new();
    let mut host_memory = HostMemory::new();
    let mut guest_memory =
        memory::GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);

    let input1 = types::Excuse::DogAte;
    let input2 = types::Excuse::Traffic;
    let input2_loc = 0;
    let input3 = types::Excuse::Sleeping;
    let input3_loc = 4;
    let input4 = types::Excuse::DogAte;
    let input4_loc = 8;
    let input4_ptr_loc = 12;

    *guest_memory
        .ptr_mut(input2_loc)
        .expect("input2 ptr")
        .as_ref_mut()
        .expect("input2 ref_mut") = input2;

    *guest_memory
        .ptr_mut(input3_loc)
        .expect("input3 ptr")
        .as_ref_mut()
        .expect("input3 ref_mut") = input3;

    *guest_memory
        .ptr_mut(input4_loc)
        .expect("input4 ptr")
        .as_ref_mut()
        .expect("input4 ref_mut") = input4;

    *guest_memory
        .ptr_mut(input4_ptr_loc)
        .expect("input4 ptr ptr")
        .as_ref_mut()
        .expect("input4 ptr ref_mut") = input4_loc;

    let baz_err = foo::baz(
        &mut ctx,
        &mut guest_memory,
        input1.into(),
        input2_loc as i32,
        input3_loc as i32,
        input4_ptr_loc as i32,
    );
    assert_eq!(baz_err, types::Errno::Ok.into());

    // Implementation of baz writes input3 to the input2_loc:
    let written_to_input2_loc: i32 = *guest_memory
        .ptr(input2_loc)
        .expect("input2 ptr")
        .as_ref()
        .expect("input2 ref");

    assert_eq!(written_to_input2_loc, input3.into());

    // Implementation of baz writes input2_loc to input4_ptr_loc:
    let written_to_input4_ptr: u32 = *guest_memory
        .ptr(input4_ptr_loc)
        .expect("input4_ptr_loc ptr")
        .as_ref()
        .expect("input4_ptr_loc ref");

    assert_eq!(written_to_input4_ptr, input2_loc);
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
    let mut host_memory = HostMemory::new();
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
