use memory::GuestRef;

generate::from_witx!({
    witx: ["tests/test.witx"],
    ctx: WasiCtx,
});

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
        input1: types::Excuse,
        input2_ptr: ::memory::GuestPtrMut<types::Excuse>,
        input3_ptr: ::memory::GuestPtr<types::Excuse>,
        input4_ptr_ptr: ::memory::GuestPtrMut<::memory::GuestPtr<types::Excuse>>,
    ) -> Result<(), types::Errno> {
        println!("BAZ input1 {:?}", input1);
        // Read enum value from mutable:
        let mut input2_ref: ::memory::GuestRefMut<types::Excuse> =
            input2_ptr.as_ref_mut().map_err(|e| {
                eprintln!("input2_ptr error: {}", e);
                types::Errno::InvalidArg
            })?;
        let input2: types::Excuse = *input2_ref;
        println!("input2 {:?}", input2);

        // Read enum value from immutable ptr:
        let input3 = *input3_ptr.as_ref().map_err(|e| {
            eprintln!("input3_ptr error: {}", e);
            types::Errno::InvalidArg
        })?;
        println!("input3 {:?}", input3);

        // Write enum to mutable ptr:
        *input2_ref = input3;
        println!("wrote to input2_ref {:?}", input3);

        // Read ptr value from mutable ptr:
        let input4_ptr: ::memory::GuestPtr<types::Excuse> =
            input4_ptr_ptr.read_ptr_from_guest().map_err(|e| {
                eprintln!("input4_ptr_ptr error: {}", e);
                types::Errno::InvalidArg
            })?;

        // Read enum value from that ptr:
        let input4: types::Excuse = *input4_ptr.as_ref().map_err(|e| {
            eprintln!("input4_ptr error: {}", e);
            types::Errno::InvalidArg
        })?;
        println!("input4 {:?}", input4);

        // Write ptr value to mutable ptr:
        input4_ptr_ptr.write_ptr_to_guest(&input2_ptr.as_immut());

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
    let mut host_memory = HostMemory::new();
    let mut guest_memory =
        memory::GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);

    let input = types::PairInts {
        first: 1,
        second: 2,
    };

    let input_loc = 0;
    let return_loc = 8;

    *guest_memory
        .ptr_mut(input_loc)
        .expect("input ptr")
        .as_ref_mut()
        .expect("input ref_mut") = input.first;
    *guest_memory
        .ptr_mut(input_loc + 4)
        .expect("input ptr")
        .as_ref_mut()
        .expect("input ref_mut") = input.second;
    let sum_err = foo::sum_of_pair(
        &mut ctx,
        &mut guest_memory,
        input_loc as i32,
        return_loc as i32,
    );

    assert_eq!(sum_err, types::Errno::Ok.into());

    let return_val: i64 = *guest_memory
        .ptr(return_loc)
        .expect("return ptr")
        .as_ref()
        .expect("return ref");

    assert_eq!(return_val, input.first as i64 + input.second as i64);
}

#[test]
fn sum_of_pair_of_ptrs() {
    let mut ctx = WasiCtx::new();
    let mut host_memory = HostMemory::new();
    let mut guest_memory =
        memory::GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);

    let input_first: i32 = 1;
    let input_second: i32 = 2;

    let input_first_loc = 0;
    let input_second_loc = 4;
    let input_struct_loc: u32 = 8;
    let return_loc: u32 = 16;

    *guest_memory
        .ptr_mut(input_first_loc)
        .expect("input_first ptr")
        .as_ref_mut()
        .expect("input_first ref") = input_first;
    *guest_memory
        .ptr_mut(input_second_loc)
        .expect("input_second ptr")
        .as_ref_mut()
        .expect("input_second ref") = input_second;

    *guest_memory
        .ptr_mut(input_struct_loc)
        .expect("input_struct ptr")
        .as_ref_mut()
        .expect("input_struct ref") = input_first_loc;
    *guest_memory
        .ptr_mut(input_struct_loc + 4)
        .expect("input_struct ptr")
        .as_ref_mut()
        .expect("input_struct ref") = input_second_loc;

    let res = foo::sum_of_pair_of_ptrs(
        &mut ctx,
        &mut guest_memory,
        input_struct_loc as i32,
        return_loc as i32,
    );

    assert_eq!(res, types::Errno::Ok.into());

    let doubled: i64 = *guest_memory
        .ptr(return_loc)
        .expect("return ptr")
        .as_ref()
        .expect("return ref");

    assert_eq!(doubled, (input_first as i64) + (input_second as i64));
}
