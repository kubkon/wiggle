use proptest::prelude::*;
use wiggle_runtime::{GuestArray, GuestError, GuestPtr, GuestPtrMut};
use wiggle_test::{impl_errno, HostMemory, MemArea, WasiCtx};

wiggle_generate::from_witx!({
    witx: ["tests/wasi.witx"],
    ctx: WasiCtx,
});

impl wiggle_runtime::GuestErrorType for types::Errno {
    type Context = WasiCtx;
    fn success() -> types::Errno {
        <types::Errno>::Success
    }
    fn from_error(e: GuestError, ctx: &mut WasiCtx) -> types::Errno {
        eprintln!("GUEST ERROR: {:?}", e);
        ctx.guest_errors.push(e);
        types::Errno::Io
    }
}
