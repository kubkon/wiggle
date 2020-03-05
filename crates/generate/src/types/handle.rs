use crate::names::Names;

use proc_macro2::TokenStream;
use quote::quote;
use witx::Layout;

pub(super) fn define_handle(
    names: &Names,
    name: &witx::Id,
    h: &witx::HandleDatatype,
) -> TokenStream {
    let ident = names.type_(name);
    let size = h.mem_size_align().size as u32;
    let align = h.mem_size_align().align as usize;
    quote! {
        #[derive(Copy, Clone, Debug, ::std::hash::Hash, Eq, PartialEq)]
        pub struct #ident(u32);

        impl From<#ident> for u32 {
            fn from(e: #ident) -> u32 {
                e.0
            }
        }

        impl From<#ident> for i32 {
            fn from(e: #ident) -> i32 {
                e.0 as i32
            }
        }

        impl From<u32> for #ident {
            fn from(e: u32) -> #ident {
                #ident(e)
            }
        }
        impl From<i32> for #ident {
            fn from(e: i32) -> #ident {
                #ident(e as u32)
            }
        }

        impl ::std::fmt::Display for #ident {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}({})", stringify!(#ident), self.0)
            }
        }

        impl<'a> wiggle_runtime::GuestType<'a> for #ident {
            fn guest_size() -> u32 {
                #size
            }

            fn guest_align() -> usize {
                #align
            }

            fn validate(location: &wiggle_runtime::GuestPtr<'a, #ident>) -> Result<*mut u8, wiggle_runtime::GuestError> {
                let host_ptr =
                    location.mem()
                        .validate_size_align(location.offset(), Self::guest_align(), Self::guest_size())?;
                Ok(host_ptr)
            }

            fn read(location: &wiggle_runtime::GuestPtr<'a, #ident>) -> Result<#ident, wiggle_runtime::GuestError> {
                let _ = Self::validate(location)?;
                Ok(#ident(u32::read(&location.cast())?))
            }

            fn write(location: &wiggle_runtime::GuestPtr<'_, Self>, val: Self) -> Result<(), wiggle_runtime::GuestError> {
                u32::write(&location.cast(), val.0)
            }
        }
    }
}
