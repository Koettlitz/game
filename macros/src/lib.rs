use std::path::PathBuf;

use build::{
    asset_enum::{derive_enum_file_name, derive_enum_type_name},
    asset_set::AssetSetArgs,
    from_def::{derive_def_type_name, from_def_trait, generate_conversion_for, generate_def_for},
    is_self, resolve_crate_name,
    spec::{SpecArgs, create_spec_impl},
};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{ToTokens, quote};
use syn::{
    DeriveInput, Ident, Item, ItemEnum, ItemStruct, LitStr, Token, Type, parse, parse_macro_input,
    punctuated::Punctuated,
};

/// Generates an implementation of [`engine::asset::AssetPathSpec`] for the annotated struct or
/// enum.
#[proc_macro_attribute]
pub fn asset_spec(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = match syn::parse(item.clone()) {
        Ok(item) => item,
        Err(e) => {
            let error = e.to_compile_error();
            let item: proc_macro2::TokenStream = item.into();
            return quote! {
                #item
                #error
            }
            .into();
        }
    };
    let type_ident = match &item {
        Item::Struct(ItemStruct { ident, .. }) => ident,
        Item::Enum(ItemEnum { ident, .. }) => ident,
        _ => {
            let error = syn::Error::new_spanned(
                &item,
                "resolver attribute is only valid for structs and enums",
            )
            .to_compile_error();
            return quote! {
                #item
                #error
            }
            .into();
        }
    };
    let args = parse_macro_input!(attr as SpecArgs);
    let spec_impl = match create_spec_impl(type_ident, &args) {
        Ok(resolver_impl) => resolver_impl,
        Err(e) => e.to_compile_error(),
    };
    quote! {
        #item

        #spec_impl
    }
    .into()
}

/// Generates an implementation of [`engine::asset::HasResolver`] for the annotated type.
/// The `engine::asset::AssetResolver` type is included based on the provided `base_path` via
/// `include!(concat!(env!("OUT_DIR"), path))`, where the `path` points to a `.rs` file generated
/// by the `build.rs`.
#[proc_macro_attribute]
pub fn asset_set(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let input_struct = parse_macro_input!(item as ItemStruct);
    let args: AssetSetArgs = match parse(attrs) {
        Ok(args) => args,
        Err(e) => {
            let error = e.to_compile_error();
            return quote! {
                #input_struct
                #error
            }
            .into();
        }
    };
    let base_path = PathBuf::from(args.base_path.value());
    let enum_file_path = match derive_enum_file_name(&base_path) {
        Ok(p) => p,
        Err(e) => {
            return syn::Error::new(args.base_path.span(), e.to_string())
                .to_compile_error()
                .into();
        }
    };
    let enum_file_path_lit = LitStr::new(
        &format!("/{}", enum_file_path.to_string_lossy()),
        Span::call_site(),
    );
    let resolver_enum_include = quote! {
        include!(concat!(env!("OUT_DIR"), #enum_file_path_lit));
    };
    let enum_type_name = match derive_enum_type_name(&base_path) {
        Ok(n) => n,
        Err(e) => {
            return syn::Error::new(args.base_path.span(), e.to_string())
                .to_compile_error()
                .into();
        }
    };
    let enum_type: Type = match syn::parse_str(&enum_type_name) {
        Ok(enum_type) => enum_type,
        Err(e) => return e.to_compile_error().into(),
    };
    let engine_crate = match resolve_crate_name("engine") {
        Ok(engine_crate) => engine_crate,
        Err(e) => return e.to_compile_error().into(),
    };
    let struct_ident = &input_struct.ident;
    let (impl_generics, ty_generics, where_clause) = input_struct.generics.split_for_impl();
    let has_resolver_impl = quote! {
        impl #impl_generics #engine_crate::asset::HasResolver for #struct_ident #ty_generics
            #where_clause
        {
            type Resolver = #enum_type;

        }
    };
    quote! {
        #input_struct

        #resolver_enum_include

        #has_resolver_impl
    }
    .into()
}

/// Implements the trait engine::asset::FromDef for the annotated struct or enum.
/// The type `FromDef::Def` can be provided by the additional attributes
/// `#[def_type(DefType)]`. If this attribute is omitted, a DefType is generated.
///
/// There are the the following ways to use this macro:
///     1. `#[def_type(Self)]`
///     2. `#[def_type(CustomType)]` provides a custom type to be used.
///     That type needs to have the same number of fields with the same names as Self.
///     The field types must match the corresponing fields type in Self in terms
///     of the Self fields FromDef::Def type.
///     3. If the additional `#[def_type]` attribute is not provided at all this macro generates a
///        def type.
///     All fields must implement FromDef tho.
#[proc_macro_derive(FromDef, attributes(def_type))]
pub fn from_def(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let engine_crate = match resolve_crate_name("engine") {
        Ok(engine_crate) => engine_crate,
        Err(e) => return e.into_compile_error().into(),
    };
    let bevy_crate = match resolve_crate_name("bevy") {
        Ok(bevy_crate) => bevy_crate,
        Err(e) => return e.into_compile_error().into(),
    };
    let load_context_var_ident = Ident::new("ctx", Span::call_site());
    let input_ident = &input.ident;
    let from_def_trait = from_def_trait(&engine_crate);
    let def_var_ident = Ident::new("def", Span::call_site());

    let mut def_type: Option<syn::Type> = None;
    for attribute in &input.attrs {
        if attribute.path().is_ident("def_type") {
            def_type = match attribute.parse_args() {
                Ok(ty) => Some(ty),
                Err(e) => return e.to_compile_error().into(),
            };
        }
    }
    let (generated_def, def_type, conversion_impl) = match def_type {
        None => {
            let def_type_name = derive_def_type_name(&input_ident.to_string());
            let def_type = match syn::parse_str(&def_type_name) {
                Ok(def_type) => def_type,
                Err(e) => return e.to_compile_error().into(),
            };
            let generated_def = match generate_def_for(&input, &engine_crate, &def_type) {
                Ok(def) => def,
                Err(e) => return e.to_compile_error().into(),
            };
            let conversion_impl = match generate_conversion_for(
                &input,
                &engine_crate,
                &def_type,
                &def_var_ident,
                &load_context_var_ident,
            ) {
                Ok(cimpl) => cimpl,
                Err(e) => return e.to_compile_error().into(),
            };
            (Some(generated_def), def_type, conversion_impl)
        }
        Some(def_type) if is_self(&def_type) => (None, def_type, def_var_ident.to_token_stream()),
        Some(def_type) => {
            let conversion_impl = match generate_conversion_for(
                &input,
                &engine_crate,
                &def_type,
                &def_var_ident,
                &load_context_var_ident,
            ) {
                Ok(cimpl) => cimpl,
                Err(e) => return e.to_compile_error().into(),
            };
            (None, def_type, conversion_impl)
        }
    };
    let macro_result = quote! {
        #generated_def

        impl #from_def_trait for #input_ident {
            type Def = #def_type;
            type Error = #engine_crate::asset::FromDefError;

            fn from_def(
                #def_var_ident: Self::Def,
                #load_context_var_ident: &mut #bevy_crate::asset::LoadContext<'_>,
            ) -> std::result::Result<Self, Self::Error> {
                Ok(#conversion_impl)
            }
        }
    }
    .into();
    macro_result
}

#[proc_macro]
pub fn from_def_self(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input with Punctuated<Type, Token![,]>::parse_terminated);
    let engine_crate = match resolve_crate_name("engine") {
        Ok(c) => c,
        Err(e) => return e.to_compile_error().into(),
    };
    let bevy_crate = match resolve_crate_name("engine") {
        Ok(c) => c,
        Err(e) => return e.to_compile_error().into(),
    };
    let from_def_trait = from_def_trait(&engine_crate);
    let mut impls = Vec::with_capacity(input.len());
    for ident in input {
        let impl_block = quote! {
            impl #from_def_trait for #ident {
                type Def = Self;
                type Error = #engine_crate::asset::FromDefError;

                fn from_def(
                    def: Self::Def,
                    _: &mut #bevy_crate::asset::LoadContext<'_>,
                ) -> Result<Self, Self::Error> {
                    Ok(def)
                }
            }
        };
        impls.push(impl_block);
    }

    quote! {
        #(#impls)*
    }
    .into()
}
