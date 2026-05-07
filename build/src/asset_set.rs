use syn::{
    Ident, LitStr, Token,
    parse::{Parse, ParseStream},
};

pub struct AssetSetArgs {
    pub base_path: LitStr,
    pub progress_name: Option<LitStr>,
}

impl Parse for AssetSetArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut base_path: Option<LitStr> = None;
        let mut progress_name: Option<LitStr> = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;

            match ident.to_string().as_str() {
                "progress_name" => {
                    input.parse::<Token![=]>()?;
                    let lit: LitStr = input.parse()?;
                    progress_name = Some(lit);
                }

                "base_path" => {
                    input.parse::<Token![=]>()?;
                    let lit: LitStr = input.parse()?;
                    base_path = Some(lit);
                }
                _ => {
                    return Err(syn::Error::new(
                        ident.span(),
                        "Unknown parameter. Expected `progress_name` or `base_path`.",
                    ));
                }
            }
            // optional trailing comma
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        let base_path =
            base_path.ok_or_else(|| syn::Error::new(input.span(), "`base_path` is required"))?;

        Ok(AssetSetArgs {
            progress_name,
            base_path,
        })
    }
}
