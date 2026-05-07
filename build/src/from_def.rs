use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

pub use def_generation::*;
pub use game_asset_impl::*;

mod def_generation {
    use proc_macro2::{Span, TokenStream};
    use quote::{ToTokens, quote};
    use syn::{
        AngleBracketedGenericArguments, Data, DataEnum, DataStruct, DeriveInput, Field, Fields,
        FieldsNamed, FieldsUnnamed, GenericArgument, Generics, Ident, PathArguments, PathSegment,
        Type, TypePath, Visibility, parse2,
    };

    use super::from_def_trait;
    pub fn generate_def_for(
        derive_input: &DeriveInput,
        engine_crate: impl ToTokens,
        def_type: &syn::Type,
    ) -> Result<TokenStream, syn::Error> {
        let def_type_definition = match &derive_input.data {
            Data::Struct(input_struct) => generate_def_for_struct(
                input_struct,
                &derive_input.vis,
                &def_type,
                &derive_input.generics,
                engine_crate,
            ),
            Data::Enum(input_enum) => generate_def_for_enum(
                &input_enum,
                &derive_input.vis,
                &def_type,
                &derive_input.generics,
                engine_crate,
            ),
            Data::Union(_) => Err(syn::Error::new_spanned(
                derive_input,
                "unions are not supported",
            )),
        }?;
        Ok(quote! {
            #[derive(serde::Serialize, serde::Deserialize)]
            #def_type_definition
        })
    }

    fn generate_def_for_struct(
        input_struct: &DataStruct,
        vis: &Visibility,
        def_type: &syn::Type,
        generics: &Generics,
        engine_crate: impl ToTokens,
    ) -> Result<TokenStream, syn::Error> {
        let mut fields = input_struct.fields.clone();
        match &mut fields {
            Fields::Named(FieldsNamed { named, .. }) => substitute_handles(named, &engine_crate)?,
            Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
                substitute_handles(unnamed, &engine_crate)?
            }
            Fields::Unit => {}
        };
        let semi_token = input_struct.semi_token;
        Ok(quote! {
            #vis struct #def_type #generics #fields #semi_token
        })
    }

    fn generate_def_for_enum(
        input_enum: &DataEnum,
        vis: &Visibility,
        def_type: &syn::Type,
        generics: &Generics,
        engine_crate: impl ToTokens,
    ) -> Result<TokenStream, syn::Error> {
        let mut variants = input_enum.variants.clone();
        for variant in &mut variants {
            substitute_handles(&mut variant.fields, &engine_crate)?;
        }
        let variants = variants.into_iter();
        Ok(quote! {
            #vis enum #def_type #generics {
                #(#variants,)*
            }
        })
    }

    fn substitute_handles<'a>(
        fields: impl IntoIterator<Item = &'a mut Field>,
        engine_crate: impl ToTokens,
    ) -> Result<(), syn::Error> {
        for field in fields {
            if let Type::Path(TypePath { ref mut path, .. }) = field.ty {
                substitute_handle(path)?;
            }
            let field_type = &field.ty;
            let game_asset_trait = from_def_trait(&engine_crate);
            field.ty = parse2(quote!(<#field_type as #game_asset_trait>::Def))?;
        }
        Ok(())
    }

    fn substitute_handle(path: &mut syn::Path) -> Result<(), syn::Error> {
        let last_segment = match path.segments.last_mut() {
            Some(last_segment) => last_segment,
            None => {
                return Err(syn::Error::new_spanned(
                    &path,
                    "dude, what is this path without any segments? I can't...",
                ));
            }
        };
        if last_segment.arguments.is_none() {
            return Ok(());
        }
        let PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) =
            &mut last_segment.arguments
        else {
            return Ok(());
        };
        if last_segment.ident == "Handle" {
            *last_segment = PathSegment {
                ident: Ident::new("String", Span::call_site()),
                arguments: PathArguments::None,
            };
            Ok(())
        } else {
            for generic_arg in args {
                let GenericArgument::Type(Type::Path(TypePath { path, .. })) = generic_arg else {
                    continue;
                };
                substitute_handle(path)?;
            }
            Ok(())
        }
    }

    #[cfg(test)]
    mod test {
        use super::generate_def_for;
        use quote::quote;
        use syn::{DeriveInput, ItemStruct, parse2};

        #[test]
        fn test_def_generation() {
            let input_struct = quote! {
                struct TestAsset<T: ops::Add> {
                    name: String,
                    fancy: Vec<Rc<RefCell<T::Output>>>,
                    handle: Handle<HurensohnAsset<'_>>,
                    nested: Vec<Rc<RefCell<Handle<T::Output>>>>,
                }
            }
            .into();
            let derive_input: DeriveInput = parse2(input_struct).unwrap();
            let engine_crate = quote!(engine);
            let def_type = syn::parse_str("TestDef").unwrap();
            let generated = generate_def_for(&derive_input, &engine_crate, &def_type).unwrap();
            let expected = quote! {
                struct TestDef<T: ops::Add> {
                    name: <String as engine::assets::FromDef>::Def,
                    fancy: <Vec<Rc<RefCell<T::Output>>> as engine::assets::FromDef>::Def,
                    handle: <String as engine::assets::FromDef>::Def,
                    nested: <Vec<Rc<RefCell<String>>> as engine::assets::FromDef>::Def
                }
            };
            let generated: ItemStruct = parse2(generated).unwrap();
            let expected: ItemStruct = parse2(expected).unwrap();
            let generated = quote!(#generated).to_string();
            let expected = quote!(#expected).to_string();
            assert_eq!(generated, expected);
        }
    }
}

mod game_asset_impl {

    use proc_macro2::{Span, TokenStream};
    use quote::{ToTokens, quote};
    use syn::{
        Data, DataEnum, DataStruct, DeriveInput, Field, Fields, FieldsNamed, FieldsUnnamed, Ident,
        Type, spanned::Spanned,
    };

    use super::from_def_trait;

    struct FromDefImplContext {
        pub engine_crate: TokenStream,
        pub def_var_ident: TokenStream,
        pub load_context_var_ident: TokenStream,
    }

    impl FromDefImplContext {
        fn new(
            engine_crate: impl ToTokens,
            def_var_ident: impl ToTokens,
            load_context_var_ident: impl ToTokens,
        ) -> Self {
            Self {
                engine_crate: engine_crate.to_token_stream(),
                def_var_ident: def_var_ident.to_token_stream(),
                load_context_var_ident: load_context_var_ident.to_token_stream(),
            }
        }
    }

    pub fn generate_conversion_for(
        derive_input: &DeriveInput,
        engine_crate: impl ToTokens,
        def_type: &syn::Type,
        def_var_ident: impl ToTokens,
        load_context_var_ident: impl ToTokens,
    ) -> Result<TokenStream, syn::Error> {
        let ctx = FromDefImplContext::new(engine_crate, def_var_ident, load_context_var_ident);
        match &derive_input.data {
            Data::Struct(input_struct) => generate_conversion_for_struct(input_struct, &ctx),
            Data::Enum(input_enum) => generate_conversion_for_enum(input_enum, def_type, &ctx),
            Data::Union(_) => Err(syn::Error::new(
                derive_input.span(),
                "def to asset conversion generation not supported for unions",
            )),
        }
    }

    fn generate_conversion_for_struct(
        input_struct: &DataStruct,
        ctx: &FromDefImplContext,
    ) -> Result<TokenStream, syn::Error> {
        Ok(match &input_struct.fields {
            Fields::Unit => quote!(Self),
            Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
                let def_var_ident = &ctx.def_var_ident;
                let field_conversions = unnamed.iter().enumerate().map(|(field_index, field)| {
                    let field_ident = syn::Index::from(field_index);
                    let field_access = quote!(#def_var_ident.#field_ident);
                    generate_field_conversion(field, field_access, ctx)
                });
                quote!(Self( #(#field_conversions),* ))
            }
            Fields::Named(FieldsNamed { named, .. }) => {
                let def_var_ident = &ctx.def_var_ident;
                let field_conversions = named.iter().map(|field| {
                    let field_ident = &field.ident;
                    let field_access = quote!(#def_var_ident.#field_ident);
                    generate_field_conversion(field, field_access, ctx)
                });
                quote!(Self { #(#field_conversions),* })
            }
        })
    }

    fn generate_conversion_for_enum(
        input_enum: &DataEnum,
        def_type: &Type,
        ctx: &FromDefImplContext,
    ) -> Result<TokenStream, syn::Error> {
        let mut variant_conversions = Vec::new();
        for variant in input_enum.variants.iter() {
            let variant_ident = &variant.ident;
            let variant_conversion = match &variant.fields {
                Fields::Unit => {
                    quote!(#def_type::#variant_ident => Self::#variant_ident)
                }
                Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
                    let fields: Vec<_> = unnamed
                        .iter()
                        .enumerate()
                        .map(|(field_index, field)| {
                            let ident = generate_field_name_for_unnamed(field_index, field.span());
                            (field, ident)
                        })
                        .collect();
                    let fields_destructured = fields.iter().map(|(_, ident)| ident);
                    let field_conversions = fields
                        .iter()
                        .map(|(field, ident)| generate_field_conversion(field, ident, ctx));
                    quote!(#def_type::#variant_ident( #(#fields_destructured),* ) => Self::#variant_ident( #(#field_conversions),* ))
                }
                Fields::Named(FieldsNamed { named, .. }) => {
                    let fields_destructured = named.iter().map(|field| &field.ident);
                    let field_conversions = named.iter().map(|field| {
                        let field_access = &field.ident;
                        generate_field_conversion(field, field_access, ctx)
                    });
                    quote!(#def_type::#variant_ident { #(#fields_destructured),* } => Self::#variant_ident { #(#field_conversions),* })
                }
            };
            variant_conversions.push(variant_conversion);
        }

        let variant_conversions = variant_conversions.into_iter();
        let def_var_ident = &ctx.def_var_ident;
        Ok(quote! {
            match #def_var_ident {
                #(#variant_conversions),*
            }
        })
    }

    fn generate_field_conversion(
        field: &Field,
        field_access: impl ToTokens,
        ctx: &FromDefImplContext,
    ) -> TokenStream {
        let colon = &field.colon_token;
        let field_type = &field.ty;
        let from_def_trait = from_def_trait(&ctx.engine_crate);
        let field_ident = &field.ident;
        let ctx_var_ident = &ctx.load_context_var_ident;
        quote! {
            #field_ident #colon <#field_type as #from_def_trait>::from_def(
                #field_access,
                #ctx_var_ident
            )?
        }
    }

    fn generate_field_name_for_unnamed(field_index: usize, field_span: Span) -> Ident {
        Ident::new(&format!("field{field_index}"), field_span)
    }
}

pub fn from_def_trait(engine_crate: impl ToTokens) -> TokenStream {
    quote!(#engine_crate::asset::FromDef)
}

pub fn derive_def_type_name(asset_type_name: &str) -> String {
    let prefix = asset_type_name
        .strip_suffix("Asset")
        .unwrap_or(asset_type_name);
    format!("{prefix}Def")
}
