use std::borrow::Cow;

use build::{
    asset_set::{AssetSetArgs, create_asset_set_impl},
    from_def::{derive_def_type_name, from_def_trait, generate_conversion_for, generate_def_for},
    resolve_crate_name,
    resolver::{ResolverArgs, create_resolver_impl},
};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{ToTokens, quote};
use syn::{
    DeriveInput, Ident, Item, ItemEnum, ItemStruct, Token, Type, parse, parse_macro_input,
    punctuated::Punctuated,
};

#[proc_macro_attribute]
pub fn resolver(attr: TokenStream, item: TokenStream) -> TokenStream {
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
    let args = parse_macro_input!(attr as ResolverArgs);
    let resolver_impl = match create_resolver_impl(type_ident, &args) {
        Ok(resolver_impl) => resolver_impl,
        Err(e) => e.to_compile_error(),
    };
    quote! {
        #item

        #resolver_impl
    }
    .into()
}

#[proc_macro_derive(FileAsset)]
pub fn file_asset(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let ident = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    match resolve_crate_name("engine") {
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
    let mut errors = Vec::new();
    let resolver_args = ResolverArgs {
        base_path: Cow::Borrowed(&args.base_path),
        extension: Cow::Borrowed(&args.extension),
        asset_type: Cow::Borrowed(&args.asset_type),
    };

    let resolver_impl = match create_resolver_impl(&input_struct.ident, &resolver_args) {
        Ok(resolver_impl) => Some(resolver_impl),
        Err(e) => {
            errors.push(e.to_compile_error());
            None
        }
    };
    let asset_set_impl = match create_asset_set_impl(&args, &input_struct, &syn::parse_quote!(Self))
    {
        Ok(asset_set_impl) => Some(asset_set_impl),
        Err(e) => {
            errors.push(e.to_compile_error());
            None
        }
    };
    quote! {
        #input_struct

        #(#errors)*

        #resolver_impl

        #asset_set_impl
    }
    .into()
}

#[proc_macro_derive(FromDef)]
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
    let def = match generate_def_for(&input, &engine_crate) {
        Ok(def) => def,
        Err(e) => return e.to_compile_error().into(),
    };
    let load_context_var_ident = Ident::new("ctx", Span::call_site());
    let ident = &input.ident;
    let game_asset_trait = from_def_trait(&engine_crate);
    let def_var_ident = Ident::new("def", Span::call_site());
    let (def_type, conversion_impl) = if def.is_some() {
        let def_type_name = derive_def_type_name(&ident.to_string());
        let def_type_ident = Ident::new(&def_type_name, Span::call_site());
        let conversion_impl = match generate_conversion_for(
            &input,
            &engine_crate,
            &def_var_ident,
            &load_context_var_ident,
            quote!(R),
        ) {
            Ok(cimpl) => cimpl,
            Err(e) => return e.to_compile_error().into(),
        };
        (def_type_ident, conversion_impl)
    } else {
        let self_type = Ident::new("Self", Span::call_site());
        (self_type, def_var_ident.to_token_stream())
    };
    let macro_result = quote! {
        #def
        impl #game_asset_trait for #ident {
            type Def = #def_type;
            type Error = #bevy_crate::asset::ParseAssetPathError;

            fn from_def<R: #engine_crate::assets::AssetResolver>(
                #def_var_ident: Self::Def,
                #load_context_var_ident: &mut #bevy_crate::asset::LoadContext<'_>,
            ) -> Result<Self, Self::Error> {
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
    let bevy_crate = match resolve_crate_name("bevy") {
        Ok(c) => c,
        Err(e) => return e.to_compile_error().into(),
    };
    let from_def_trait = from_def_trait(&engine_crate);
    let mut impls = Vec::with_capacity(input.len());
    for ident in input {
        let impl_block = quote! {
            impl #from_def_trait for #ident {
                type Def = Self;
                type Error = #bevy_crate::asset::ParseAssetPathError;

                fn from_def<R: #engine_crate::assets::AssetResolver>(
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
