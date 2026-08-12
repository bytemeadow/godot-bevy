use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, LitStr, Token, parse2};

/// Build a Godot `PropertyHint::ENUM` string from a fieldless Rust enum.
pub fn derive_godot_hint_string(input: TokenStream) -> syn::Result<TokenStream> {
    let input = parse2::<DeriveInput>(input)?;
    let enum_name = &input.ident;
    let Data::Enum(data) = &input.data else {
        return Err(Error::new_spanned(
            &input.ident,
            "GodotHintString can only be derived for enums",
        ));
    };

    let mut labels = Vec::new();
    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(Error::new_spanned(
                &variant.fields,
                "GodotHintString only supports fieldless enum variants",
            ));
        }

        let mut skip = false;
        let mut label = None;
        for attr in &variant.attrs {
            if !attr.path().is_ident("godot_hint") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("skip") {
                    if skip {
                        return Err(meta.error("duplicate `skip`"));
                    }
                    skip = true;
                    return Ok(());
                }
                if meta.path.is_ident("label") {
                    if label.is_some() {
                        return Err(meta.error("duplicate `label`"));
                    }
                    meta.input.parse::<Token![=]>()?;
                    label = Some(meta.input.parse::<LitStr>()?);
                    return Ok(());
                }
                Err(meta.error("expected `skip` or `label = \"...\"`"))
            })?;
        }

        if skip && label.is_some() {
            return Err(Error::new_spanned(
                &variant.ident,
                "`skip` and `label` cannot be used together",
            ));
        }
        if !skip {
            labels.push(
                label.unwrap_or_else(|| {
                    LitStr::new(&variant.ident.to_string(), variant.ident.span())
                }),
            );
        }
    }

    let hint_string = labels
        .iter()
        .map(LitStr::value)
        .collect::<Vec<_>>()
        .join(",");
    let hint_string = LitStr::new(&hint_string, enum_name.span());

    Ok(quote! {
        impl #enum_name {
            pub const GODOT_HINT_STRING: &'static str = #hint_string;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::derive_godot_hint_string;
    use quote::{ToTokens, quote};
    use syn::DeriveInput;

    #[test]
    fn uses_variant_names_by_default() {
        let input: DeriveInput = syn::parse2(quote! {
            enum WeaponKind { Hands, Knife }
        })
        .unwrap();
        let output = derive_godot_hint_string(input.into_token_stream()).unwrap();
        assert!(
            output
                .to_string()
                .contains("GODOT_HINT_STRING : & 'static str = \"Hands,Knife\"")
        );
    }

    #[test]
    fn supports_custom_labels_and_skipped_variants() {
        let input: DeriveInput = syn::parse2(quote! {
            enum WeaponKind {
                Hands,
                #[godot_hint(label = "Assault Rifle")]
                Rifle,
                #[godot_hint(skip)]
                Removed,
            }
        })
        .unwrap();
        let output = derive_godot_hint_string(input.into_token_stream()).unwrap();
        assert!(
            output
                .to_string()
                .contains("GODOT_HINT_STRING : & 'static str = \"Hands,Assault Rifle\"")
        );
    }

    #[test]
    fn rejects_data_variants() {
        let input: DeriveInput = syn::parse2(quote! {
            enum Invalid { Good, Bad(u8) }
        })
        .unwrap();
        assert!(derive_godot_hint_string(input.into_token_stream()).is_err());
    }
}
