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

// pub fn generate_path_consts(
//     asset_root: &Path,
//     asset_paths: &HashMap<PathBuf, Vec<PathBuf>>,
// ) -> String {
//     let mut output = String::new();
//     let mut asset_paths: Vec<_> = asset_paths.iter().collect();
//     asset_paths.sort_by_key(|(d, _)| *d);
//     for (dir, contents) in asset_paths {
//         let mut names = Vec::new();
//         for path in contents {
//             let Some(file_name) = path.file_name() else {
//                 continue;
//             };
//             let name = file_name
//                 .to_string_lossy()
//                 .split('.')
//                 .next()
//                 .unwrap()
//                 .to_string();
//             names.push(name);
//         }
//         let const_name = &dir
//             .strip_prefix(asset_root)
//             .unwrap_or_else(|e| {
//                 panic!(
//                     "could not strip prefix asset root {} from asset dir path {} - {e}",
//                     asset_root.display(),
//                     dir.display()
//                 )
//             })
//             .to_string_lossy()
//             .replace(['/', '\\', '.'], "_")
//             .to_uppercase();
//         output.push_str(&format!("pub const {const_name}: &[&str] = &["));
//         names.sort();
//         for name in names {
//             output.push_str(&format!("\n    \"{name}\","));
//         }
//         output.push_str(&format!("\n];\n"));
//     }
//     output
// }
