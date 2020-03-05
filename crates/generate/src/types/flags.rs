use super::{atom_token, int_repr_tokens};
use crate::names::Names;

use proc_macro2::{Literal, TokenStream};
use quote::quote;
use std::convert::TryFrom;

pub(super) fn define_flags(names: &Names, name: &witx::Id, f: &witx::FlagsDatatype) -> TokenStream {
    let ident = names.type_(&name);
    let repr = int_repr_tokens(f.repr);
    let abi_repr = atom_token(match f.repr {
        witx::IntRepr::U8 | witx::IntRepr::U16 | witx::IntRepr::U32 => witx::AtomType::I32,
        witx::IntRepr::U64 => witx::AtomType::I64,
    });

    let mut flag_constructors = vec![];
    let mut all_values = 0;
    for (i, f) in f.flags.iter().enumerate() {
        let name = names.flag_member(&f.name);
        let value = 1u128
            .checked_shl(u32::try_from(i).expect("flag value overflow"))
            .expect("flag value overflow");
        let value_token = Literal::u128_unsuffixed(value);
        flag_constructors.push(quote!(pub const #name: #ident = #ident(#value_token)));
        all_values += value;
    }
    let all_values_token = Literal::u128_unsuffixed(all_values);

    let ident_str = ident.to_string();

    quote! {
        #[repr(transparent)]
        #[derive(Copy, Clone, Debug, ::std::hash::Hash, Eq, PartialEq)]
        pub struct #ident(#repr);

        impl #ident {
            #(#flag_constructors);*;
            pub const EMPTY_FLAGS: #ident = #ident(0 as #repr);
            pub const ALL_FLAGS: #ident = #ident(#all_values_token);

            pub fn contains(&self, other: &#ident) -> bool {
                !*self & *other == Self::EMPTY_FLAGS
            }

            // Reading and validation are nearly the same thing for flags, so we define one private
            // helper method that we use for GuestValue::read and GuestValue::validate
            fn validate_read(ptr: &wiggle_runtime::GuestPtr<#ident>) -> Result<(*mut u8, #ident), wiggle_runtime::GuestError> {
                let host_ptr =
                    ptr.mem()
                        .validate_size_align(ptr.offset(), Self::guest_align(), Self::guest_size())?;
                use std::convert::TryFrom;
                use wiggle_runtime::GuestType;
                let reprval = #repr::read(&ptr.cast())?;
                let value = #ident::try_from(reprval)?;
                Ok((host_ptr, value))
            }
        }

        impl ::std::fmt::Display for #ident {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}({:#b})", #ident_str, self.0)
            }
        }

        impl ::std::ops::BitAnd for #ident {
            type Output = Self;
            fn bitand(self, rhs: Self) -> Self::Output {
                #ident(self.0 & rhs.0)
            }
        }

        impl ::std::ops::BitAndAssign for #ident {
            fn bitand_assign(&mut self, rhs: Self) {
                *self = *self & rhs
            }
        }

        impl ::std::ops::BitOr for #ident {
            type Output = Self;
            fn bitor(self, rhs: Self) -> Self::Output {
                #ident(self.0 | rhs.0)
            }
        }

        impl ::std::ops::BitOrAssign for #ident {
            fn bitor_assign(&mut self, rhs: Self) {
                *self = *self | rhs
            }
        }

        impl ::std::ops::BitXor for #ident {
            type Output = Self;
            fn bitxor(self, rhs: Self) -> Self::Output {
                #ident(self.0 ^ rhs.0)
            }
        }

        impl ::std::ops::BitXorAssign for #ident {
            fn bitxor_assign(&mut self, rhs: Self) {
                *self = *self ^ rhs
            }
        }

        impl ::std::ops::Not for #ident {
            type Output = Self;
            fn not(self) -> Self::Output {
                #ident(!self.0)
            }
        }

        impl ::std::convert::TryFrom<#repr> for #ident {
            type Error = wiggle_runtime::GuestError;
            fn try_from(value: #repr) -> Result<Self, wiggle_runtime::GuestError> {
                if #repr::from(!#ident::ALL_FLAGS) & value != 0 {
                    Err(wiggle_runtime::GuestError::InvalidFlagValue(stringify!(#ident)))
                } else {
                    Ok(#ident(value))
                }
            }
        }

        impl ::std::convert::TryFrom<#abi_repr> for #ident {
            type Error = wiggle_runtime::GuestError;
            fn try_from(value: #abi_repr) -> Result<#ident, wiggle_runtime::GuestError> {
                #ident::try_from(value as #repr)
            }
        }

        impl From<#ident> for #repr {
            fn from(e: #ident) -> #repr {
                e.0
            }
        }

        impl From<#ident> for #abi_repr {
            fn from(e: #ident) -> #abi_repr {
                #repr::from(e) as #abi_repr
            }
        }

        impl<'a> wiggle_runtime::GuestType<'a> for #ident {
            fn guest_size() -> u32 {
                #repr::guest_size()
            }

            fn guest_align() -> usize {
                #repr::guest_align()
            }

            fn validate(location: &wiggle_runtime::GuestPtr<'a, Self>) -> Result<*mut u8, wiggle_runtime::GuestError> {
                let (validate, _) = Self::validate_read(location)?;
                Ok(validate)
            }

            fn read(location: &wiggle_runtime::GuestPtr<#ident>) -> Result<#ident, wiggle_runtime::GuestError> {
                let (_, read) = Self::validate_read(location)?;
                Ok(read)
            }

            fn write(location: &wiggle_runtime::GuestPtr<'_, #ident>, val: Self) -> Result<(), wiggle_runtime::GuestError> {
                let val: #repr = #repr::from(val);
                #repr::write(&location.cast(), val)
            }
        }
    }
}
