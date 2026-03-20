use build::{
    asset_set::{AssetSetArgs, create_asset_set_impl},
    resolve_crate_path,
};
use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, ItemStruct, parse_macro_input};

#[proc_macro_derive(FileAsset)]
pub fn file_asset(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let ident = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    match resolve_crate_path("engine") {
        Ok(engine_crate) => quote! {
            impl #impl_generics #engine_crate::assets::folder::FileAsset
            for #ident #ty_generics
            #where_clause
            {}
        }
        .into(),
        Err(e) => e.into_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn asset_set(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attrs as AssetSetArgs);
    let input_struct = parse_macro_input!(item as ItemStruct);
    match create_asset_set_impl(args, &input_struct) {
        Ok(impl_block) => quote! {
            #input_struct
            #impl_block
        }
        .into(),
        Err(e) => {
            let compile_error = e.to_compile_error();
            let expanded = quote! {
                #input_struct
                #compile_error
            };
            expanded.into()
        }
    }
}
