use heck::{CamelCase, MixedCase, ShoutySnakeCase};
use proc_macro2::{Delimiter, Group, Literal, TokenStream, TokenTree};
use quote::{format_ident, quote};
use std::convert::TryFrom;

const WITX_PATH: &'static str = "crates/WASI/phases/snapshot/witx/wasi_snapshot_preview1.witx";

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Mode {
    Host,
    Wasi32,
    Wasi,
}

impl Mode {
    pub fn include_target_types(&self) -> bool {
        match self {
            Mode::Host | Mode::Wasi32 => true,
            Mode::Wasi => false,
        }
    }
}

pub fn gen() -> TokenStream {
    let mut output = TokenStream::new();
    let doc = match witx::load(&[&WITX_PATH]) {
        Ok(doc) => doc,
        Err(e) => {
            panic!("error opening file {}: {}", WITX_PATH, e);
        }
    };

    gen_datatypes(&mut output, &doc, Mode::Wasi);
    // gen_datatypes(&mut output, &doc, Mode::Wasi32);
    // gen_datatypes(&mut output, &doc, Mode::Host);

    output
}

fn gen_datatypes(output: &mut TokenStream, doc: &witx::Document, mode: Mode) {
    for namedtype in doc.typenames() {
        if mode.include_target_types() != namedtype_has_target_size(&namedtype) {
            continue;
        }

        gen_datatype(output, mode, &namedtype);
    }
}

fn gen_datatype(output: &mut TokenStream, mode: Mode, namedtype: &witx::NamedType) {
    let wasi_name = format_ident!("{}", namedtype.name.as_str().to_camel_case());
    match &namedtype.tref {
        witx::TypeRef::Name(alias_to) => {
            let to = tref_tokens(mode, &alias_to.tref);
            output.extend(quote!(pub type #wasi_name = #to;));
        }
        witx::TypeRef::Value(v) => match &**v {
            witx::Type::Enum(e) => {
                let repr = int_repr_tokens(e.repr);
                output.extend(quote!(#[repr(#repr)]));
                output
                    .extend(quote!(#[derive(Copy, Clone, Debug, std::hash::Hash, Eq, PartialEq)]));

                let mut inner = TokenStream::new();
                for variant in &e.variants {
                    let value_name = if namedtype.name.as_str() == "errno" {
                        // FIXME discussion point!
                        format_ident!("E{}", variant.name.as_str().to_mixed_case())
                    } else {
                        format_ident!("{}", variant.name.as_str().to_camel_case())
                    };
                    inner.extend(quote!(#value_name,));
                }

                output.extend(quote!(pub enum #wasi_name {
                    #inner
                }));
            }
            witx::Type::Int(_) => {} // TODO
            witx::Type::Flags(f) => {
                let repr = int_repr_tokens(f.repr);
                output.extend(quote!(#[repr(transparent)]));
                output
                    .extend(quote!(#[derive(Copy, Clone, Debug, std::hash::Hash, Eq, PartialEq)]));
                output.extend(quote!(pub struct #wasi_name(#repr);));
                // TODO
                // Since `Flags` are represented by a "transparent" struct, we should probably
                // auto-generate `from_raw(raw: #repr)` method or similar

                let mut inner = TokenStream::new();
                for (index, flag) in f.flags.iter().enumerate() {
                    let value_name = format_ident!("{}", flag.name.as_str().to_shouty_snake_case());
                    let flag_value = Literal::u128_unsuffixed(
                        1u128
                            .checked_shl(u32::try_from(index).expect("flag value overflow"))
                            .expect("flag value overflow"),
                    );
                    inner.extend(
                        quote!(pub const #value_name: #wasi_name = #wasi_name(#flag_value);),
                    );
                }

                output.extend(quote!(impl #wasi_name {
                    #inner
                }));
            }
            witx::Type::Struct(s) => {
                output.extend(quote!(#[repr(C)]));
                // Types which contain unions can't trivially implement Debug,
                // Hash, or Eq, because the type itself doesn't record which
                // union member is active.
                if struct_has_union(&s) {
                    output.extend(quote!(#[derive(Copy, Clone)]));
                    output.extend(quote!(#[allow(missing_debug_implementations)]));
                } else {
                    output.extend(quote!(#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]));
                }

                output.extend(quote!(pub struct #wasi_name));

                let mut inner = TokenStream::new();
                for member in &s.members {
                    let member_name = format_ident!("r#{}", member.name.as_str());
                    let member_type = tref_tokens(mode, &member.tref);
                    inner.extend(quote!(pub #member_name: #member_type,));
                }
                let braced = Group::new(Delimiter::Brace, inner);
                output.extend(TokenStream::from(TokenTree::Group(braced)));
            }
            witx::Type::Union(u) => {
                output.extend(quote!(#[repr(C)]));
                output.extend(quote!(#[derive(Copy, Clone)]));
                output.extend(quote!(#[allow(missing_debug_implementations)]));

                output.extend(quote!(pub union #wasi_name));

                let mut inner = TokenStream::new();
                for variant in &u.variants {
                    let variant_name = format_ident!("r#{}", variant.name.as_str());
                    let variant_type = tref_tokens(mode, &variant.tref);
                    inner.extend(quote!(pub #variant_name: #variant_type,));
                }
                let braced = Group::new(Delimiter::Brace, inner);
                output.extend(TokenStream::from(TokenTree::Group(braced)));
            }
            witx::Type::Handle(_h) => {
                output.extend(quote!(pub type #wasi_name = u32;));
            }
            witx::Type::Builtin(b) => {
                if namedtype.name.as_str() == "size" {
                    match mode {
                        Mode::Host => output.extend(quote!(pub type #wasi_name = usize;)),
                        Mode::Wasi => panic!("size has target-specific size"),
                        Mode::Wasi32 => output.extend(quote!(pub type #wasi_name = u32;)),
                    }
                } else {
                    let b_type = builtin_tokens(mode, *b);
                    output.extend(quote!(pub type #wasi_name = #b_type;));
                }
            }
            witx::Type::Pointer { .. }
            | witx::Type::ConstPointer { .. }
            | witx::Type::Array { .. } => {
                let tref_tokens = tref_tokens(mode, &namedtype.tref);
                output.extend(quote!(pub type #wasi_name = #tref_tokens;));
            }
        },
    }
}

fn int_repr_tokens(int_repr: witx::IntRepr) -> TokenStream {
    match int_repr {
        witx::IntRepr::U8 => quote!(u8),
        witx::IntRepr::U16 => quote!(u16),
        witx::IntRepr::U32 => quote!(u32),
        witx::IntRepr::U64 => quote!(u64),
    }
}

fn builtin_tokens(mode: Mode, builtin: witx::BuiltinType) -> TokenStream {
    match builtin {
        witx::BuiltinType::String => match mode {
            Mode::Host => quote!((*const u8, usize)),
            Mode::Wasi => panic!("strings have target-specific size"),
            Mode::Wasi32 => quote!((u32, u32)),
        },
        witx::BuiltinType::U8 => quote!(u8),
        witx::BuiltinType::U16 => quote!(u16),
        witx::BuiltinType::U32 => quote!(u32),
        witx::BuiltinType::U64 => quote!(u64),
        witx::BuiltinType::S8 => quote!(i8),
        witx::BuiltinType::S16 => quote!(i16),
        witx::BuiltinType::S32 => quote!(i32),
        witx::BuiltinType::S64 => quote!(i64),
        witx::BuiltinType::F32 => quote!(f32),
        witx::BuiltinType::F64 => quote!(f64),
        witx::BuiltinType::Char8 => quote!(char),
        witx::BuiltinType::USize => quote!(usize),
    }
}

fn tref_tokens(mode: Mode, tref: &witx::TypeRef) -> TokenStream {
    match tref {
        witx::TypeRef::Name(n) => TokenStream::from(TokenTree::Ident(format_ident!(
            "{}",
            n.name.as_str().to_camel_case()
        ))),
        witx::TypeRef::Value(v) => match &**v {
            witx::Type::Builtin(b) => builtin_tokens(mode, *b),
            witx::Type::Pointer(pointee) => {
                let pointee = tref_tokens(mode, pointee);
                match mode {
                    Mode::Host => quote!(*mut #pointee),
                    Mode::Wasi => panic!("pointers have target-specific size"),
                    Mode::Wasi32 => quote!(u32),
                }
            }
            witx::Type::ConstPointer(pointee) => {
                let pointee = tref_tokens(mode, pointee);
                match mode {
                    Mode::Host => quote!(*const #pointee),
                    Mode::Wasi => panic!("pointers have target-specific size"),
                    Mode::Wasi32 => quote!(u32),
                }
            }
            witx::Type::Array(element) => {
                let element_name = tref_tokens(mode, element);
                match mode {
                    Mode::Host => quote!((*const #element_name, usize)),
                    Mode::Wasi => panic!("arrays have target-specific size"),
                    Mode::Wasi32 => quote!((u32, u32)),
                }
            }
            t => panic!("cannot give name to anonymous type {:?}", t),
        },
    }
}

/// Test whether the given struct contains any union members.
fn struct_has_union(s: &witx::StructDatatype) -> bool {
    s.members.iter().any(|member| match &*member.tref.type_() {
        witx::Type::Union { .. } => true,
        witx::Type::Struct(s) => struct_has_union(&s),
        _ => false,
    })
}

/// Test whether the type referred to has a target-specific size.
fn tref_has_target_size(tref: &witx::TypeRef) -> bool {
    match tref {
        witx::TypeRef::Name(nt) => namedtype_has_target_size(&nt),
        witx::TypeRef::Value(t) => type_has_target_size(&t),
    }
}

/// Test whether the given named type has a target-specific size.
fn namedtype_has_target_size(nt: &witx::NamedType) -> bool {
    if nt.name.as_str() == "size" {
        true
    } else {
        tref_has_target_size(&nt.tref)
    }
}

/// Test whether the given type has a target-specific size.
fn type_has_target_size(ty: &witx::Type) -> bool {
    match ty {
        witx::Type::Builtin(witx::BuiltinType::String) => true,
        witx::Type::Pointer { .. } | witx::Type::ConstPointer { .. } => true,
        witx::Type::Array(elem) => tref_has_target_size(elem),
        witx::Type::Struct(s) => s.members.iter().any(|m| tref_has_target_size(&m.tref)),
        witx::Type::Union(u) => u.variants.iter().any(|v| tref_has_target_size(&v.tref)),
        _ => false,
    }
}