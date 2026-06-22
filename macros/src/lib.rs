use std::path::PathBuf;

use build::{
    CratePath,
    asset_enum::{derive_enum_file_name, derive_enum_type_name},
    asset_set::AssetSetArgs,
};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{ItemStruct, LitStr, Type, parse, parse_macro_input};

/// Generates an implementation of [`HasResolver`] for the annotated type.
/// The [`AssetResolver`] type is included based on the provided `base_path` via
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
    let elf_module = match CratePath::try_from("bevy_elf") {
        Ok(asset_module) => asset_module,
        Err(e) => return e.to_compile_error().into(),
    };
    let asset_set_module = match CratePath::try_from("engine::asset::set") {
        Ok(asset_set_module) => asset_set_module,
        Err(e) => return e.to_compile_error().into(),
    };
    let struct_ident = &input_struct.ident;
    let (impl_generics, ty_generics, where_clause) = input_struct.generics.split_for_impl();
    let has_resolver_impl = quote! {
        impl #impl_generics #elf_module::HasResolver for #struct_ident #ty_generics
            #where_clause
        {
            type Resolver = #elf_module::StaticResolverAdapter<#enum_type>;

            fn resolver() -> Self::Resolver {
                #elf_module::StaticResolverAdapter::<#enum_type>::default()
            }
        }
    };
    let has_resolver_set_impl = quote! {
        impl #asset_set_module::HasResolverSet for #struct_ident #ty_generics
            #where_clause
    {
            type ResolverSet = #enum_type;
        }
    };
    let base_path = args.base_path;
    // TODO: include asset source prefix in base_path
    let asset_spec_impl = quote! {
        impl #elf_module::AssetPathSpec for #enum_type {
            const BASE_PATH: &'static str = #base_path;
        }
    };
    quote! {
        #input_struct

        #resolver_enum_include

        #has_resolver_impl

        #has_resolver_set_impl

        #asset_spec_impl
    }
    .into()
}
