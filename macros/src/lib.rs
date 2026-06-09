use std::path::PathBuf;

use build::{
    ASSET_MODULE_PATH, CratePath,
    asset_enum::{derive_enum_file_name, derive_enum_type_name},
    asset_set::AssetSetArgs,
    from_def::{
        DefTransformResult, derive_def_type_name, from_def_trait, generate_def_for,
        generate_def_transform,
    },
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
                "asset_spec attribute is only valid for structs and enums",
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
    let asset_module = match CratePath::try_from(ASSET_MODULE_PATH) {
        Ok(asset_module) => asset_module,
        Err(e) => return e.to_compile_error().into(),
    };
    quote! {
        #item

        #spec_impl

        impl #asset_module::HasResolver for #type_ident {
            type Resolver = #asset_module::ResolverSpec<Self>;

            fn resolver() -> Self::Resolver {
                #asset_module::ResolverSpec::<Self>::default()
            }
        }
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
    let asset_module = match CratePath::try_from("engine::asset") {
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
        impl #impl_generics #asset_module::HasResolver for #struct_ident #ty_generics
            #where_clause
        {
            type Resolver = #asset_module::StaticResolverAdapter<#enum_type>;

            fn resolver() -> Self::Resolver {
                #asset_module::StaticResolverAdapter::<#enum_type>::default()
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
        impl #asset_module::AssetPathSpec for #enum_type {
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

/// Implements the trait [`engine::asset::FromDef`] for the annotated struct or enum.
/// The def type (`FromDef::Def`) can be provided by the additional attribute
/// `#[def_type(DefType)]`. If this attribute is omitted, a DefType is generated.
///
/// There are the following ways to specify the def_type:
///     1. `#[def_type(Self)]` where no conversion is necessary, because the serializable type is
///        also the runtime type. `from_def()` just returns `Self` as is.
///     2. `#[def_type(CustomType)]` to provide a custom serializable def type to be used.
///        That type needs to have a corresponing field with the same name for each field in `Self`
///        that should be converted.
///        The field types must match the corresponing fields type in `Self` in terms
///        of its `FromDef::Def` type.
///     3. If the additional `#[def_type]` attribute is omitted this macro generates a
///        def type.
/// All fields included must implement FromDef though.
///
/// It is possible to influence resolution and def type generation by using the
/// `#[from_def(...)]` attribute on the fields directly:
///
/// `#[from_def(default)]` will use the [`std::default::Default`] trait to construct a value, so
/// it omits the field in the generated def type and also skips resolution completely.
///
/// `#[from_def(implicit)]` will omit the field in the generated def type and use the same id
/// as of self to resolve the file name. The `implicit` option can be combined freely with `with_spec`.
///
/// `#[from_def(with_spec(base_path = "base/path"))]` overrides the `base_path` of the fields
/// type used for resolution. This is only relevant for types like [`bevy::asset::Handle<T>`] or [`engine::asset::AssetRef<T>`]
///
/// Alternatively to specifying a `base_path` you can use
/// `#[from_def(with_spec(sub_path = "foo"))]` to to make the field being resolved relatively
/// to the current path (the `base_path` used to resolve the containing type).
///
/// Use `#[expose_resolver]` on a field to generate a function on the type containing the field
/// which exposes the resolver. The name is derived from the field name (e.g.
/// `MyAsset::foo_resolver()` for the field `foo`)
#[proc_macro_derive(FromDef, attributes(def_type, from_def, expose_resolver))]
pub fn from_def(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let asset_module = match CratePath::try_from(ASSET_MODULE_PATH) {
        Ok(asset_module) => asset_module,
        Err(e) => return e.into_compile_error().into(),
    };
    let bevy_crate = match resolve_crate_name("bevy") {
        Ok(bevy_crate) => bevy_crate,
        Err(e) => return e.into_compile_error().into(),
    };
    let load_context_var_ident = Ident::new("ctx", Span::call_site());
    let input_ident = &input.ident;
    let from_def_trait = match from_def_trait() {
        Ok(from_def_trait) => from_def_trait,
        Err(e) => return e.into_compile_error().into(),
    };
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
    let (generated_def, def_type, def_transform) = match def_type {
        None => {
            let def_type_name = derive_def_type_name(&input_ident.to_string());
            let def_type = match syn::parse_str(&def_type_name) {
                Ok(def_type) => def_type,
                Err(e) => return e.to_compile_error().into(),
            };
            let generated_def = match generate_def_for(&input, &def_type) {
                Ok(def) => def,
                Err(e) => return e.to_compile_error().into(),
            };
            let def_transform = match generate_def_transform(
                &input,
                &def_type,
                &def_var_ident,
                &load_context_var_ident,
            ) {
                Ok(def_transform) => def_transform,
                Err(e) => return e.to_compile_error().into(),
            };
            (Some(generated_def), def_type, def_transform)
        }
        Some(def_type) if is_self(&def_type) => (
            None,
            def_type,
            DefTransformResult {
                transformation: def_var_ident.to_token_stream(),
                resolver_fns: Vec::new(),
            },
        ),
        Some(def_type) => {
            let def_transform = match generate_def_transform(
                &input,
                &def_type,
                &def_var_ident,
                &load_context_var_ident,
            ) {
                Ok(cimpl) => cimpl,
                Err(e) => return e.to_compile_error().into(),
            };
            (None, def_type, def_transform)
        }
    };
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let (transformation, resolver_fns) = (def_transform.transformation, def_transform.resolver_fns);
    let resolver_fns = if resolver_fns.is_empty() {
        None
    } else {
        Some(quote! {
            impl #impl_generics #input_ident #ty_generics #where_clause {
                #(#resolver_fns)*
            }
        })
    };

    quote! {
        #generated_def

        impl #impl_generics #from_def_trait #ty_generics for #input_ident #where_clause {
            type Def = #def_type;
            type Error = #asset_module::FromDefError;

            fn from_def(
                #def_var_ident: Self::Def,
                #load_context_var_ident: &mut #bevy_crate::asset::LoadContext<'_>,
            ) -> std::result::Result<Self, Self::Error> {
                Ok(#transformation)
            }
        }

        #resolver_fns
    }
    .into()
}

#[proc_macro]
pub fn from_def_self(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input with Punctuated<Type, Token![,]>::parse_terminated);
    let bevy_crate = match resolve_crate_name("bevy") {
        Ok(c) => c,
        Err(e) => return e.to_compile_error().into(),
    };
    let from_def_trait = match from_def_trait() {
        Ok(from_def_trait) => from_def_trait,
        Err(e) => return e.into_compile_error().into(),
    };
    let asset_module = match CratePath::try_from(ASSET_MODULE_PATH) {
        Ok(asset_module) => asset_module,
        Err(e) => return e.into_compile_error().into(),
    };
    let mut impls = Vec::with_capacity(input.len());
    for ident in input {
        let impl_block = quote! {
            impl #from_def_trait for #ident {
                type Def = Self;
                type Error = #asset_module::FromDefError;

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
