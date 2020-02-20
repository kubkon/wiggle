use super::array::{GuestArray, GuestArrayRef};
use crate::GuestError;
use std::{fmt, ops::Deref};

pub struct GuestString<'a> {
    pub(super) array: GuestArray<'a, u8>,
}

impl<'a> fmt::Debug for GuestString<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GuestString {{ array: {:?} }}", self.array)
    }
}

impl<'a> GuestString<'a> {
    pub fn as_ref(&self) -> Result<GuestStringRef<'a>, GuestError> {
        let ref_ = self.array.as_ref()?;
        Ok(GuestStringRef { ref_ })
    }

    pub fn to_string(&self) -> Result<String, GuestError> {
        Ok(self.as_ref()?.as_str()?.to_owned())
    }
}

pub struct GuestStringRef<'a> {
    pub(super) ref_: GuestArrayRef<'a, u8>,
}

impl<'a> fmt::Debug for GuestStringRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GuestStringRef {{ ref_: {:?} }}", self.ref_)
    }
}

impl<'a> GuestStringRef<'a> {
    pub fn as_str(&self) -> Result<&str, GuestError> {
        let bytes = self.ref_.deref();
        let len = bytes.len();
        std::str::from_utf8(&bytes[..len - 1]).map_err(|_| GuestError::InvalidUtf8)
    }
}

#[cfg(test)]
mod test {
    use super::super::{
        ptr::{GuestPtr, GuestPtrMut},
        GuestError, GuestMemory,
    };

    #[repr(align(4096))]
    struct HostMemory {
        buffer: [u8; 4096],
    }

    impl HostMemory {
        pub fn new() -> Self {
            Self { buffer: [0; 4096] }
        }
        pub fn as_mut_ptr(&mut self) -> *mut u8 {
            self.buffer.as_mut_ptr()
        }
        pub fn len(&self) -> usize {
            self.buffer.len()
        }
    }

    #[test]
    fn valid_utf8() {
        let mut host_memory = HostMemory::new();
        // poison all host's memory so that we test for null-termination
        host_memory.buffer = [1; 4096];
        let guest_memory = GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);
        // write string into memory
        let mut ptr: GuestPtrMut<u8> = guest_memory.ptr_mut(0).expect("ptr mut to start of string");
        let input_str = "cześć WASI!";
        let mut bytes = input_str.as_bytes().to_vec();
        bytes.push(b'\0');
        for byte in bytes {
            let mut ref_mut = ptr.as_ref_mut().expect("valid deref");
            *ref_mut = byte;
            ptr = ptr.elem(1).expect("next ptr");
        }
        // read the string as GuestString
        let ptr: GuestPtr<u8> = guest_memory.ptr(0).expect("ptr to start of string");
        let guest_string = ptr.string().expect("valid null-terminated string");
        let as_ref = guest_string.as_ref().expect("deref");
        assert_eq!(as_ref.as_str().expect("valid UTF-8"), input_str);
    }

    #[test]
    fn invalid_utf8() {
        let mut host_memory = HostMemory::new();
        let guest_memory = GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);
        // write string into memory
        let mut ptr: GuestPtrMut<u8> = guest_memory.ptr_mut(0).expect("ptr mut to start of string");
        let input_str = "cześć WASI!";
        let mut bytes = input_str.as_bytes().to_vec();
        bytes.push(b'\0');
        // insert 0xFE which is an invalid UTF-8 byte
        bytes[5] = 0xfe;
        for byte in bytes {
            let mut ref_mut = ptr.as_ref_mut().expect("valid deref");
            *ref_mut = byte;
            ptr = ptr.elem(1).expect("next ptr");
        }
        // read the string as GuestString
        let ptr: GuestPtr<u8> = guest_memory.ptr(0).expect("ptr to start of string");
        let guest_string = ptr.string().expect("valid null-terminated string");
        let as_ref = guest_string.as_ref().expect("deref");
        assert_eq!(as_ref.as_str(), Err(GuestError::InvalidUtf8));
    }
}
