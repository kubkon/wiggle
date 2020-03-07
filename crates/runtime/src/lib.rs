use std::cell::Cell;
use std::fmt;
use std::marker;
use std::rc::Rc;
use std::slice;
use std::str;
use std::sync::Arc;

mod borrow;
mod error;
mod guest_type;
mod region;

pub use borrow::GuestBorrows;
pub use error::GuestError;
pub use guest_type::{GuestErrorType, GuestType, GuestTypeTransparent};
pub use region::Region;

/// A trait which abstracts how to get at the region of host memory taht
/// contains guest memory.
///
/// All `GuestPtr` types will contain a handle to this trait, signifying where
/// the pointer is actually pointing into. This type will need to be implemented
/// for the host's memory storage object.
///
/// # Safety
///
/// Safety around this type is tricky, and the trait is `unsafe` since there are
/// a few contracts you need to uphold to implement this type correctly and have
/// everything else in this crate work out safely.
///
/// The most important method of this trait is the `base` method. This returns,
/// in host memory, a pointer and a length. The pointer should point to valid
/// memory for the guest to read/write for the length contiguous bytes
/// afterwards.
///
/// The region returned by `base` must not only be valid, however, but it must
/// be valid for "a period of time before the guest is reentered". This isn't
/// exactly well defined but the general idea is that `GuestMemory` is allowed
/// to change under our feet to accomodate instructions like `memory.grow` or
/// other guest modifications. Memory, however, cannot be changed if the guest
/// is not reentered or if no explicitly action is taken to modify the guest
/// memory.
///
/// This provides the guarantee that host pointers based on the return value of
/// `base` have a dynamic period for which they are valid. This time duration
/// must be "somehow nonzero in length" to allow users of `GuestMemory` and
/// `GuestPtr` to safely read and write interior data.
///
/// # Using Raw Pointers
///
/// Methods like [`GuestMemory::base`] or [`GuestPtr::as_raw`] will return raw
/// pointers to use. Returning raw pointers is significant because it shows
/// there are hazards with using the returned pointers, and they can't blanket
/// be used in a safe fashion. It is possible to use these pointers safely, but
/// any usage needs to uphold a few guarantees.
///
/// * Whenever a `*mut T` is accessed or modified, it must be guaranteed that
///   since the pointer was originally obtained the guest memory wasn't
///   relocated in any way. This means you can't call back into the guest, call
///   other arbitrary functions which might call into the guest, etc. The
///   problem here is that the guest could execute instructions like
///   `memory.grow` which would invalidate the raw pointer. If, however, after
///   you acquire `*mut T` you only execute your own code and it doesn't touch
///   the guest, then `*mut T` is still guaranteed to point to valid code.
///
/// * Furthermore, Rust's aliasing rules must still be upheld. For example you
///   can't have two `&mut T` types that point to the area or overlap in any
///   way. This in particular becomes an issue when you're dealing with multiple
///   `GuestPtr` types. If you want to simultaneously work with them then you
///   need to dynamically validate that you're either working with them all in a
///   shared fashion (e.g. as if they were `&T`) or you must verify that they do
///   not overlap to work with them as `&mut T`.
///
/// Note that safely using the raw pointers is relatively difficult. This crate
/// strives to provide utilities to safely work with guest pointers so long as
/// the previous guarantees are all upheld. If advanced operations are done with
/// guest pointers it's recommended to be extremely cautious and thoroughly
/// consider possible ramifications with respect to this API before codifying
/// implementation details.
pub unsafe trait GuestMemory {
    /// Returns the base allocation of this guest memory, located in host
    /// memory.
    ///
    /// A pointer/length pair are returned to signify where the guest memory
    /// lives in the host, and how many contiguous bytes the memory is valid for
    /// after the returned pointer.
    ///
    /// Note that there are safety guarantees about this method that
    /// implementations must uphold, and for more details see the
    /// [`GuestMemory`] documentation.
    fn base(&self) -> (*mut u8, u32);

    /// Validates a guest-relative pointer given various attributes, and returns
    /// the corresponding host pointer.
    ///
    /// * `offset` - this is the guest-relative pointer, an offset from the
    ///   base.
    /// * `align` - this is the desired alignment of the guest pointer, and if
    ///   successful the host pointer will be guaranteed to have this alignment.
    /// * `len` - this is the number of bytes, after `offset`, that the returned
    ///   pointer must be valid for.
    ///
    /// This function will guarantee that the returned pointer is in-bounds of
    /// `base`, *at this time*, for `len` bytes and has alignment `align`. If
    /// any guarantees are not upheld then an error will be returned.
    ///
    /// Note that the returned pointer is an unsafe pointer. This is not safe to
    /// use in general because guest memory can be relocated. Additionally the
    /// guest may be modifying/reading memory as well. Consult the
    /// [`GuestMemory`] documentation for safety information about using this
    /// returned pointer.
    fn validate_size_align(
        &self,
        offset: u32,
        align: usize,
        len: u32,
    ) -> Result<*mut u8, GuestError> {
        let (base_ptr, base_len) = self.base();
        let region = Region { start: offset, len };

        // Figure out our pointer to the start of memory
        let start = match (base_ptr as usize).checked_add(offset as usize) {
            Some(ptr) => ptr,
            None => return Err(GuestError::PtrOverflow),
        };
        // and use that to figure out the end pointer
        let end = match start.checked_add(len as usize) {
            Some(ptr) => ptr,
            None => return Err(GuestError::PtrOverflow),
        };
        // and then verify that our end doesn't reach past the end of our memory
        if end > (base_ptr as usize) + (base_len as usize) {
            return Err(GuestError::PtrOutOfBounds(region));
        }
        // and finally verify that the alignment is correct
        if start % align != 0 {
            return Err(GuestError::PtrNotAligned(region, align as u32));
        }
        Ok(start as *mut u8)
    }

    /// Convenience method for creating a `GuestPtr` at a particular offset.
    ///
    /// Note that `T` can be almost any type, and typically `offset` is a `u32`.
    /// The exception is slices and strings, in which case `offset` is a `(u32,
    /// u32)` of `(offset, length)`.
    fn ptr<'a, T>(&'a self, offset: T::Pointer) -> GuestPtr<'a, T>
    where
        Self: Sized,
        T: ?Sized + Pointee,
    {
        GuestPtr::new(self, offset)
    }
}

// Forwarding trait implementations to the original type

unsafe impl<'a, T: ?Sized + GuestMemory> GuestMemory for &'a T {
    fn base(&self) -> (*mut u8, u32) {
        T::base(self)
    }
}

unsafe impl<'a, T: ?Sized + GuestMemory> GuestMemory for &'a mut T {
    fn base(&self) -> (*mut u8, u32) {
        T::base(self)
    }
}

unsafe impl<T: ?Sized + GuestMemory> GuestMemory for Box<T> {
    fn base(&self) -> (*mut u8, u32) {
        T::base(self)
    }
}

unsafe impl<T: ?Sized + GuestMemory> GuestMemory for Rc<T> {
    fn base(&self) -> (*mut u8, u32) {
        T::base(self)
    }
}

unsafe impl<T: ?Sized + GuestMemory> GuestMemory for Arc<T> {
    fn base(&self) -> (*mut u8, u32) {
        T::base(self)
    }
}

/// A *guest* pointer into host memory.
///
/// This type represents a pointer from the guest that points into host memory.
/// Internally a `GuestPtr` contains a handle to its original [`GuestMemory`] as
/// well as the offset into the memory that the pointer is pointing at.
///
/// Presence of a [`GuestPtr`] does not imply any form of validity. Pointers can
/// be out-of-bounds, misaligned, etc. It is safe to construct a `GuestPtr` with
/// any offset at any time. Consider a `GuestPtr<T>` roughly equivalent to `*mut
/// T`, although there are a few more safety guarantees around this type.
///
/// ## Slices and Strings
///
/// Note that the type parameter does not need to implement the `Sized` trait,
/// so you can implement types such as this:
///
/// * `GuestPtr<'_, str>` - a pointer to a guest string
/// * `GuestPtr<'_, [T]>` - a pointer to a guest array
///
/// Unsized types such as this may have extra methods and won't have methods
/// like [`GuestPtr::read`] or [`GuestPtr::write`].
///
/// ## Type parameter and pointee
///
/// The `T` type parameter is largely intended for more static safety in Rust as
/// well as having a better handle on what we're pointing to. A `GuestPtr<T>`,
/// however, does not necessarily literally imply a guest pointer pointing to
/// type `T`. Instead the [`GuestType`] trait is a layer of abstraction where
/// `GuestPtr<T>` may actually be a pointer to `U` in guest memory, but you can
/// construct a `T` from a `U`.
///
/// For example `GuestPtr<GuestPtr<T>>` is a valid type, but this is actually
/// more equivalent to `GuestPtr<u32>` because guest pointers are always
/// 32-bits. That being said you can create a `GuestPtr<T>` from a `u32`.
///
/// Additionally `GuestPtr<MyEnum>` will actually delegate, typically, to and
/// implementation which loads the underlying data as `GuestPtr<u8>` (or
/// similar) and then the bytes loaded are validated to fit within the
/// definition of `MyEnum` before `MyEnum` is returned.
///
/// For more information see the [`GuestPtr::read`] and [`GuestPtr::write`]
/// methods. In general though be extremely careful about writing `unsafe` code
/// when working with a `GuestPtr` if you're not using one of the
/// already-attached helper methods.
pub struct GuestPtr<'a, T: ?Sized + Pointee> {
    mem: &'a (dyn GuestMemory + 'a),
    pointer: T::Pointer,
    _marker: marker::PhantomData<&'a Cell<T>>,
}

impl<'a, T: ?Sized + Pointee> GuestPtr<'a, T> {
    /// Creates a new `GuestPtr` from the given `mem` and `pointer` values.
    ///
    /// Note that for sized types like `u32`, `GuestPtr<T>`, etc, the `pointer`
    /// vlue is a `u32` offset into guest memory. For slices and strings,
    /// `pointer` is a `(u32, u32)` offset/length pair.
    pub fn new(mem: &'a (dyn GuestMemory + 'a), pointer: T::Pointer) -> GuestPtr<'_, T> {
        GuestPtr {
            mem,
            pointer,
            _marker: marker::PhantomData,
        }
    }

    /// Returns the offset of this pointer in guest memory.
    ///
    /// Note that for sized types this returns a `u32`, but for slices and
    /// strings it returns a `(u32, u32)` pointer/length pair.
    pub fn offset(&self) -> T::Pointer {
        self.pointer
    }

    /// Returns the guest memory that this pointer is coming from.
    pub fn mem(&self) -> &'a (dyn GuestMemory + 'a) {
        self.mem
    }

    /// Casts this `GuestPtr` type to a different type.
    ///
    /// This is a safe method which is useful for simply reinterpreting the type
    /// parameter on this `GuestPtr`. Note that this is a safe method, where
    /// again there's no guarantees about alignment, validity, in-bounds-ness,
    /// etc of the returned pointer.
    pub fn cast<U>(&self) -> GuestPtr<'a, U>
    where
        T: Pointee<Pointer = u32>,
    {
        GuestPtr::new(self.mem, self.pointer)
    }

    /// Safely read a value from this pointer.
    ///
    /// This is a fun method, and is one of the lynchpins of this
    /// implementation. The highlight here is that this is a *safe* operation,
    /// not an unsafe one like `*mut T`. This works for a few reasons:
    ///
    /// * The `unsafe` contract of the `GuestMemory` trait means that there's
    ///   always at least some backing memory for this `GuestPtr<T>`.
    ///
    /// * This does not use Rust-intrinsics to read the type `T`, but rather it
    ///   delegates to `T`'s implementation of [`GuestType`] to actually read
    ///   the underlying data. This again is a safe method, so any unsafety, if
    ///   any, must be internally documented.
    ///
    /// * Eventually what typically happens it that this bottoms out in the read
    ///   implementations for primitives types (like `i32`) which can safely be
    ///   read at any time, and then it's up to the runtime to determine what to
    ///   do with the bytes it read in a safe manner.
    ///
    /// Naturally lots of things can still go wrong, such as out-of-bounds
    /// checks, alignment checks, validity checks (e.g. for enums), etc. All of
    /// these check failures, however, are returned as a [`GuestError`] in the
    /// `Result` here, and `Ok` is only returned if all the checks passed.
    pub fn read(&self) -> Result<T, GuestError>
    where
        T: GuestType<'a>,
    {
        T::read(self)
    }

    /// Safely write a value to this pointer.
    ///
    /// This method, like [`GuestPtr::read`], is pretty crucial for the safe
    /// operation of this crate. All the same reasons apply though for why this
    /// method is safe, even eventually bottoming out in primitives like writing
    /// an `i32` which is safe to write bit patterns into memory at any time due
    /// to the guarantees of [`GuestMemory`].
    ///
    /// Like `read`, `write` can fail due to any manner of pointer checks, but
    /// any failure is returned as a [`GuestError`].
    pub fn write(&self, val: T) -> Result<(), GuestError>
    where
        T: GuestType<'a>,
    {
        T::write(self, val)
    }

    /// Performs pointer arithmetic on this pointer, moving the pointer forward
    /// `amt` slots.
    ///
    /// This will either return the resulting pointer or `Err` if the pointer
    /// arithmetic calculation would overflow around the end of the address
    /// space.
    pub fn add(&self, amt: u32) -> Result<GuestPtr<'a, T>, GuestError>
    where
        T: GuestType<'a> + Pointee<Pointer = u32>,
    {
        let offset = amt
            .checked_mul(T::guest_size())
            .and_then(|o| self.pointer.checked_add(o));
        let offset = match offset {
            Some(o) => o,
            None => return Err(GuestError::PtrOverflow),
        };
        Ok(GuestPtr::new(self.mem, offset))
    }

    /// Returns a `GuestPtr` for an array of `T`s using this pointer as the
    /// base.
    pub fn as_array(&self, elems: u32) -> GuestPtr<'a, [T]>
    where
        T: GuestType<'a> + Pointee<Pointer = u32>,
    {
        GuestPtr::new(self.mem, (self.pointer, elems))
    }
}

impl<'a, T> GuestPtr<'a, [T]> {
    /// For slices, specifically returns the relative pointer to the base of the
    /// array.
    ///
    /// This is similar to `<[T]>::as_ptr()`
    pub fn offset_base(&self) -> u32 {
        self.pointer.0
    }

    /// For slices, returns the length of the slice, in units.
    pub fn len(&self) -> u32 {
        self.pointer.1
    }

    /// Returns an iterator over interior pointers.
    ///
    /// Each item is a `Result` indicating whether it overflowed past the end of
    /// the address space or not.
    pub fn iter<'b>(
        &'b self,
    ) -> impl ExactSizeIterator<Item = Result<GuestPtr<'a, T>, GuestError>> + 'b
    where
        T: GuestType<'a>,
    {
        let base = self.as_ptr();
        (0..self.len()).map(move |i| base.add(i))
    }

    /// Attempts to read a raw `*mut [T]` pointer from this pointer, performing
    /// bounds checks and type validation.
    /// The resulting `*mut [T]` can be used as a `&mut [t]` as long as the
    /// reference is dropped before any Wasm code is re-entered.
    ///
    /// This function will return a raw pointer into host memory if all checks
    /// succeed (valid utf-8, valid pointers, etc). If any checks fail then
    /// `GuestError` will be returned.
    ///
    /// Note that the `*mut [T]` pointer is still unsafe to use in general, but
    /// there are specific situations that it is safe to use. For more
    /// information about using the raw pointer, consult the [`GuestMemory`]
    /// trait documentation.
    ///
    /// For safety against overlapping mutable borrows, the user must use the
    /// same `GuestBorrows` to create all *mut str or *mut [T] that are alive
    /// at the same time.
    pub fn as_raw(&self, bc: &mut GuestBorrows) -> Result<*mut [T], GuestError>
    where
        T: GuestTypeTransparent<'a>,
    {
        let len = match self.pointer.1.checked_mul(T::guest_size()) {
            Some(l) => l,
            None => return Err(GuestError::PtrOverflow),
        };
        let ptr =
            self.mem
                .validate_size_align(self.pointer.0, T::guest_align(), len)? as *mut T;

        bc.borrow(Region {
            start: self.pointer.0,
            len,
        })?;

        // Validate all elements in slice.
        // SAFETY: ptr has been validated by self.mem.validate_size_align
        for offs in 0..self.pointer.1 {
            T::validate(unsafe { ptr.add(offs as usize) })?;
        }

        // SAFETY: iff there are no overlapping borrows (all uses of as_raw use this same
        // GuestBorrows), its valid to construct a *mut [T]
        unsafe {
            let s = slice::from_raw_parts_mut(ptr, self.pointer.1 as usize);
            Ok(s as *mut [T])
        }
    }

    /// Returns a `GuestPtr` pointing to the base of the array for the interior
    /// type `T`.
    pub fn as_ptr(&self) -> GuestPtr<'a, T> {
        GuestPtr::new(self.mem, self.offset_base())
    }
}

impl<'a> GuestPtr<'a, str> {
    /// For strings, returns the relative pointer to the base of the string
    /// allocation.
    pub fn offset_base(&self) -> u32 {
        self.pointer.0
    }

    /// Returns the length, in bytes, of th estring.
    pub fn len(&self) -> u32 {
        self.pointer.1
    }

    /// Returns a raw pointer for the underlying slice of bytes that this
    /// pointer points to.
    pub fn as_bytes(&self) -> GuestPtr<'a, [u8]> {
        GuestPtr::new(self.mem, self.pointer)
    }

    /// Attempts to read a raw `*mut str` pointer from this pointer, performing
    /// bounds checks and utf-8 checks.
    /// The resulting `*mut str` can be used as a `&mut str` as long as the
    /// reference is dropped before any Wasm code is re-entered.
    ///
    /// This function will return a raw pointer into host memory if all checks
    /// succeed (valid utf-8, valid pointers, etc). If any checks fail then
    /// `GuestError` will be returned.
    ///
    /// Note that the `*mut str` pointer is still unsafe to use in general, but
    /// there are specific situations that it is safe to use. For more
    /// information about using the raw pointer, consult the [`GuestMemory`]
    /// trait documentation.
    ///
    /// For safety against overlapping mutable borrows, the user must use the
    /// same `GuestBorrows` to create all *mut str or *mut [T] that are alive
    /// at the same time.
    pub fn as_raw(&self, bc: &mut GuestBorrows) -> Result<*mut str, GuestError> {
        let ptr = self
            .mem
            .validate_size_align(self.pointer.0, 1, self.pointer.1)?;

        bc.borrow(Region {
            start: self.pointer.0,
            len: self.pointer.1,
        })?;

        // SAFETY: iff there are no overlapping borrows (all uses of as_raw use this same
        // GuestBorrows), its valid to construct a *mut str
        unsafe {
            let s = slice::from_raw_parts_mut(ptr, self.pointer.1 as usize);
            match str::from_utf8_mut(s) {
                Ok(s) => Ok(s),
                Err(e) => Err(GuestError::InvalidUtf8(e)),
            }
        }
    }
}

impl<T: ?Sized + Pointee> Clone for GuestPtr<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized + Pointee> Copy for GuestPtr<'_, T> {}

impl<T: ?Sized + Pointee> fmt::Debug for GuestPtr<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        T::debug(self.pointer, f)
    }
}

mod private {
    pub trait Sealed {}
    impl<T> Sealed for T {}
    impl<T> Sealed for [T] {}
    impl Sealed for str {}
}

/// Types that can be pointed to by `GuestPtr<T>`.
///
/// In essence everything can, and the only special-case is unsized types like
/// `str` and `[T]` which have special implementations.
pub trait Pointee: private::Sealed {
    #[doc(hidden)]
    type Pointer: Copy;
    #[doc(hidden)]
    fn debug(pointer: Self::Pointer, f: &mut fmt::Formatter) -> fmt::Result;
}

impl<T> Pointee for T {
    type Pointer = u32;
    fn debug(pointer: Self::Pointer, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "*guest {:#x}", pointer)
    }
}

impl<T> Pointee for [T] {
    type Pointer = (u32, u32);
    fn debug(pointer: Self::Pointer, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "*guest {:#x}/{}", pointer.0, pointer.1)
    }
}

impl Pointee for str {
    type Pointer = (u32, u32);
    fn debug(pointer: Self::Pointer, f: &mut fmt::Formatter) -> fmt::Result {
        <[u8]>::debug(pointer, f)
    }
}
