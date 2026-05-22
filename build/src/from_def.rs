use std::fmt::Debug;

pub use def_generation::*;
pub use game_asset_impl::*;
use proc_macro2::Span;
use syn::{Attribute, Ident, LitStr, Token, parenthesized, parse::Parse, spanned::Spanned};

use crate::{ASSET_MODULE_PATH, CratePath};

#[derive(Debug)]
struct FieldAttr {
    implicit: bool,
    spec: Option<FieldSpec>,
}

impl FieldAttr {
    fn validate(&self, span: Span) -> syn::Result<()> {
        if !self.implicit && self.spec.is_none() {
            Err(syn::Error::new(
                span,
                "expected at least one of `implicit` or `with_spec`",
            ))
        } else if self.implicit
            && self
                .spec
                .as_ref()
                .is_some_and(|spec| spec.extension.is_none())
        {
            Err(syn::Error::new(
                span,
                "expected `extension` on implicit field",
            ))
        } else {
            Ok(())
        }
    }
}

impl Parse for FieldAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut implicit = false;
        let mut spec: Option<FieldSpec> = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;

            match ident.to_string().as_str() {
                "implicit" => {
                    implicit = true;
                }
                "with_spec" => {
                    let spec_args;
                    parenthesized!(spec_args in input);
                    spec = Some(FieldSpec::parse(&spec_args)?);
                }
                _ => {
                    return Err(syn::Error::new(
                        ident.span(),
                        "Unknown parameter. Expected `base_path`, or `extension`",
                    ));
                }
            }
            // optional trailing comma
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self { implicit, spec })
    }
}

impl FieldAttr {
    fn parse<'a>(attrs: impl IntoIterator<Item = &'a Attribute>) -> syn::Result<Option<Self>> {
        for attr in attrs {
            if attr.path().is_ident("from_def") {
                let field_attr: Self = attr.parse_args()?;
                field_attr.validate(attr.path().span())?;
                return Ok(Some(field_attr));
            }
        }
        Ok(None)
    }
}

struct FieldSpec {
    path_kind: PathKind,
    extension: Option<LitStr>,
}

impl Debug for FieldSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref extension) = self.extension {
            write!(
                f,
                "({:?}, extension = \"{}\")",
                self.path_kind,
                extension.value()
            )
        } else {
            write!(f, "({:?})", self.path_kind)
        }
    }
}

enum PathKind {
    Root(LitStr),
    Child(Option<LitStr>),
}

impl Debug for PathKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Root(base_path) => write!(f, "base_path = \"{}\"", base_path.value()),
            Self::Child(sub_path) => match sub_path {
                Some(sub_path) => write!(f, "sub_path = \"{}\"", sub_path.value()),
                None => write!(f, "sub_path"),
            },
        }
    }
}

impl Parse for FieldSpec {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut path_kind: Option<PathKind> = None;
        let mut extension: Option<LitStr> = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;

            match ident.to_string().as_str() {
                "base_path" => {
                    input.parse::<Token![=]>()?;
                    let lit: LitStr = input.parse()?;
                    path_kind = Some(PathKind::Root(lit));
                }
                "sub_path" => {
                    let path = if input.peek(Token![=]) {
                        input.parse::<Token![=]>()?;
                        let lit: LitStr = input.parse()?;
                        Some(lit)
                    } else {
                        None
                    };
                    path_kind = Some(PathKind::Child(path));
                }
                "extension" => {
                    input.parse::<Token![=]>()?;
                    let lit: LitStr = input.parse()?;
                    extension = Some(lit);
                }
                _ => {
                    return Err(syn::Error::new(
                        ident.span(),
                        "Unknown parameter. Expected `base_path`, or `extension`",
                    ));
                }
            }
            // optional trailing comma
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        if path_kind.is_none() {
            return Err(syn::Error::new(
                input.span(),
                "either `base_path = \"base/path\"` or `sub_path [= \"sub/path\"]` is required",
            ));
        }

        Ok(Self {
            path_kind: path_kind.unwrap(),
            extension,
        })
    }
}

mod def_generation {
    use proc_macro2::TokenStream;
    use quote::quote;
    use syn::{
        Data, DataEnum, DataStruct, DeriveInput, Field, Fields, Generics, Visibility, parse2,
        punctuated::Punctuated, token::Comma,
    };

    use crate::{ASSET_MODULE_PATH, CratePath, from_def::FieldAttr};

    pub fn generate_def_for(
        derive_input: &DeriveInput,
        def_type: &syn::Type,
    ) -> Result<TokenStream, syn::Error> {
        let def_type_definition = match &derive_input.data {
            Data::Struct(input_struct) => generate_def_for_struct(
                input_struct,
                &derive_input.vis,
                &def_type,
                &derive_input.generics,
            ),
            Data::Enum(input_enum) => generate_def_for_enum(
                &input_enum,
                &derive_input.vis,
                &def_type,
                &derive_input.generics,
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
    ) -> Result<TokenStream, syn::Error> {
        let fields = match input_struct.fields.clone() {
            Fields::Named(mut named) => {
                named.named = generate_def_fields(named.named)?;
                Fields::Named(named)
            }
            Fields::Unnamed(mut unnamed) => {
                unnamed.unnamed = generate_def_fields(unnamed.unnamed)?;
                Fields::Unnamed(unnamed)
            }
            Fields::Unit => Fields::Unit,
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
    ) -> Result<TokenStream, syn::Error> {
        let mut variants = input_enum.variants.clone();
        for variant in &mut variants {
            variant.fields = match variant.fields.clone() {
                Fields::Named(mut named) => {
                    named.named = generate_def_fields(named.named)?;
                    Fields::Named(named)
                }
                Fields::Unnamed(mut unnamed) => {
                    unnamed.unnamed = generate_def_fields(unnamed.unnamed)?;
                    Fields::Unnamed(unnamed)
                }
                Fields::Unit => Fields::Unit,
            };
        }
        let variants = variants.into_iter();
        Ok(quote! {
            #vis enum #def_type #generics {
                #(#variants,)*
            }
        })
    }

    fn generate_def_fields(
        fields: Punctuated<Field, Comma>,
    ) -> syn::Result<Punctuated<Field, Comma>> {
        fields
            .into_iter()
            .filter_map(|f| generate_def_field(f).transpose())
            .collect()
    }

    fn generate_def_field(mut field: Field) -> syn::Result<Option<Field>> {
        let from_def_trait = match FieldAttr::parse(&field.attrs)? {
            Some(FieldAttr { implicit: true, .. }) => return Ok(None),
            Some(FieldAttr {
                implicit: false,
                spec: Some(_),
            }) => {
                let asset_module = CratePath::try_from(ASSET_MODULE_PATH)?;
                quote!(#asset_module::FromDefWithResolver)
            }
            _ => {
                let asset_module = CratePath::try_from(ASSET_MODULE_PATH)?;
                quote!(#asset_module::FromDef)
            }
        };
        let field_type = &field.ty;
        field.attrs.clear();
        field.ty = parse2(quote!(<#field_type as #from_def_trait>::Def))?;
        Ok(Some(field))
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
            let def_type = syn::parse_str("TestDef").unwrap();
            let generated = generate_def_for(&derive_input, &def_type).unwrap();
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
        AngleBracketedGenericArguments, Data, DataEnum, DataStruct, DeriveInput, Field, Fields,
        FieldsNamed, FieldsUnnamed, GenericArgument, Ident, PathArguments, Type, TypePath,
        spanned::Spanned,
    };

    use crate::{
        ASSET_MODULE_PATH, CratePath,
        from_def::{FieldAttr, FieldSpec, PathKind},
    };

    use super::from_def_trait;

    struct FromDefImplContext {
        pub def_var_ident: TokenStream,
        pub load_context_var_ident: TokenStream,
    }

    impl FromDefImplContext {
        fn new(def_var_ident: impl ToTokens, load_context_var_ident: impl ToTokens) -> Self {
            Self {
                def_var_ident: def_var_ident.to_token_stream(),
                load_context_var_ident: load_context_var_ident.to_token_stream(),
            }
        }
    }

    pub fn generate_def_transform(
        derive_input: &DeriveInput,
        def_type: &syn::Type,
        def_var_ident: impl ToTokens,
        load_context_var_ident: impl ToTokens,
    ) -> Result<TokenStream, syn::Error> {
        let ctx = FromDefImplContext::new(def_var_ident, load_context_var_ident);
        match &derive_input.data {
            Data::Struct(input_struct) => generate_def_transform_for_struct(input_struct, &ctx),
            Data::Enum(input_enum) => generate_def_transform_for_enum(input_enum, def_type, &ctx),
            Data::Union(_) => Err(syn::Error::new(
                derive_input.span(),
                "def to asset conversion generation not supported for unions",
            )),
        }
    }

    fn generate_def_transform_for_struct(
        input_struct: &DataStruct,
        ctx: &FromDefImplContext,
    ) -> Result<TokenStream, syn::Error> {
        Ok(match &input_struct.fields {
            Fields::Unit => quote!(Self),
            Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
                let def_var_ident = &ctx.def_var_ident;
                let field_conversions = unnamed
                    .iter()
                    .enumerate()
                    .map(|(field_index, field)| {
                        let field_ident = syn::Index::from(field_index);
                        let field_access = quote!(#def_var_ident.#field_ident);
                        generate_field_conversion(field, field_access, ctx)
                    })
                    .collect::<Result<Vec<TokenStream>, syn::Error>>()?;
                quote!(Self( #(#field_conversions),* ))
            }
            Fields::Named(FieldsNamed { named, .. }) => {
                let def_var_ident = &ctx.def_var_ident;
                let field_conversions = named
                    .iter()
                    .map(|field| {
                        let field_ident = &field.ident;
                        let field_access = quote!(#def_var_ident.#field_ident);
                        generate_field_conversion(field, field_access, ctx)
                    })
                    .collect::<Result<Vec<TokenStream>, syn::Error>>()?;
                quote!(Self { #(#field_conversions),* })
            }
        })
    }

    fn generate_def_transform_for_enum(
        input_enum: &DataEnum,
        def_type: &Type,
        ctx: &FromDefImplContext,
    ) -> Result<TokenStream, syn::Error> {
        let mut variant_conversions = Vec::new();
        for variant in input_enum.variants.iter() {
            let variant_ident = &variant.ident;
            let variant_conversion = match &variant.fields {
                Fields::Unit => quote!(#def_type::#variant_ident => Self::#variant_ident),
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
                        .map(|(field, ident)| generate_field_conversion(field, ident, ctx))
                        .collect::<Result<Vec<TokenStream>, syn::Error>>()?;
                    quote! {
                        #def_type::#variant_ident( #(#fields_destructured),* ) => Self::#variant_ident( #(#field_conversions),* )
                    }
                }
                Fields::Named(FieldsNamed { named, .. }) => {
                    let fields_destructured = named.iter().map(|field| &field.ident);
                    let field_conversions = named
                        .iter()
                        .map(|field| {
                            let field_access = &field.ident;
                            generate_field_conversion(field, field_access, ctx)
                        })
                        .collect::<Result<Vec<TokenStream>, syn::Error>>()?;
                    quote! {
                        #def_type::#variant_ident { #(#fields_destructured),* } => Self::#variant_ident { #(#field_conversions),* }
                    }
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
    ) -> Result<TokenStream, syn::Error> {
        let asset_module = CratePath::try_from(ASSET_MODULE_PATH)?;
        let colon = &field.colon_token;
        let field_type = &field.ty;
        let from_def_trait = from_def_trait()?;
        let field_ident = &field.ident;
        let ctx_var_ident = &ctx.load_context_var_ident;

        let attr = FieldAttr::parse(&field.attrs)?;
        let def_expr = if let Some(FieldAttr { implicit: true, .. }) = &attr {
            quote! {
                <<Self as #asset_module::HasResolver>::Resolver as #asset_module::AssetResolver>::to_id(
                    &<Self as #asset_module::HasResolver>::resolver(),
                    #ctx_var_ident.path().clone()
                )
            }
        } else {
            field_access.to_token_stream()
        };
        let conversion = if let Some(field_spec) = attr.as_ref().and_then(|a| a.spec.as_ref()) {
            let resolver_expr = generate_resolver_from(&field.ty, field_spec, ctx)?;

            quote! {
                #field_ident #colon <#field_type as #asset_module::FromDefWithResolver>::from_def_with_resolver(
                    #def_expr,
                    &#resolver_expr,
                    #ctx_var_ident
                )?
            }
        } else {
            quote! {
                #field_ident #colon <#field_type as #from_def_trait>::from_def(
                    #def_expr,
                    #ctx_var_ident
                )?
            }
        };

        Ok(conversion)
    }

    fn generate_resolver_from(
        field_type: &syn::Type,
        spec: &FieldSpec,
        ctx: &FromDefImplContext,
    ) -> syn::Result<TokenStream> {
        let asset_module = CratePath::try_from(ASSET_MODULE_PATH)?;
        let provider_expr = match &spec.path_kind {
            PathKind::Root(base_path) => {
                let extension = if let Some(extension) = spec.extension.as_ref() {
                    quote!(Some(#extension))
                } else {
                    quote!(None)
                };
                quote! {
                    #asset_module::DynamicPathResolver {
                        base_path: #base_path.to_string(),
                        extension: #extension,
                    }
                }
            }
            PathKind::Child(sub_path) => {
                let asset_type = extract_asset_type(field_type).ok_or_else(|| {
                    syn::Error::new(
                        field_type.span(),
                        format!(
                            "`subpath` only allowed for types, that contain a bevy::asset::Handle"
                        ),
                    )
                })?;
                let (sub_path, extension) = sub_path
                    .as_ref()
                    .map(|p| {
                        let extension = if let Some(extension) = spec.extension.as_ref() {
                            quote!(Some(#extension))
                        } else {
                            quote!(None)
                        };
                        (p.to_token_stream(), extension) })
                    .unwrap_or_else(|| {(
                            quote! {
                                <#asset_type as #asset_module::HasSpecProvider>::provider().base_path()
                            },
                            quote! {
                                <#asset_type as #asset_module::HasSpecProvider>::provider().extension()
                            }
                    )});
                let ctx_var_ident = &ctx.load_context_var_ident;
                quote! {
                    #asset_module::DynamicPathResolver::resolve_sub_path(
                        #ctx_var_ident,
                        #sub_path,
                        #extension
                    )?
                }
            }
        };

        Ok(provider_expr)
    }

    fn extract_asset_type(field_type: &syn::Type) -> Option<&syn::Type> {
        let syn::Type::Path(TypePath { path, .. }) = field_type else {
            return None;
        };
        let last_segment = path.segments.last()?;
        let PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) =
            &last_segment.arguments
        else {
            return None;
        };
        if last_segment.ident == "Handle" {
            return if let GenericArgument::Type(asset_type) = args.first()? {
                Some(asset_type)
            } else {
                None
            };
        } else {
            for generic_arg in args {
                if let GenericArgument::Type(inner) = generic_arg {
                    let result = extract_asset_type(inner);
                    if result.is_some() {
                        return result;
                    }
                }
            }
            None
        }
    }

    fn generate_field_name_for_unnamed(field_index: usize, field_span: Span) -> Ident {
        Ident::new(&format!("field{field_index}"), field_span)
    }
}

pub fn from_def_trait() -> Result<CratePath, syn::Error> {
    let path = ASSET_MODULE_PATH.to_string() + "::FromDef";
    CratePath::try_from(path.as_str())
}

pub fn derive_def_type_name(asset_type_name: &str) -> String {
    let prefix = asset_type_name
        .strip_suffix("Asset")
        .unwrap_or(asset_type_name);
    format!("{prefix}Def")
}
