use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Token, parse2};

#[derive(Clone)]
pub(crate) struct KeyValue {
    pub key: syn::Ident,
    pub value: syn::Expr,
}

impl Parse for KeyValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: syn::Ident = input.parse()?;
        let content;
        syn::parenthesized!(content in input);
        let value: syn::Expr = content.parse()?;
        Ok(KeyValue { key, value })
    }
}

pub struct GodotNodeAttrArgs {
    pub base: Option<syn::Ident>,
    pub class_name: Option<syn::Ident>,
}

/// Parses the following format:
/// ```ignore
/// base(<godot_type>), class_name(<identifier>)
/// ```
impl Parse for GodotNodeAttrArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let arguments = Punctuated::<KeyValue, Token![,]>::parse_terminated(input)?;
        let mut base = None;
        let mut class_name = None;

        for argument in arguments {
            if argument.key == "base" {
                base = Some(parse2::<syn::Ident>(argument.value.to_token_stream())?);
            } else if argument.key == "class_name" {
                class_name = Some(parse2::<syn::Ident>(argument.value.to_token_stream())?);
            } else {
                return Err(syn::Error::new(
                    argument.key.span(),
                    format!(
                        "Unknown parameter: `{}`. Expected `base` or `class_name`.",
                        argument.key
                    ),
                ));
            }
        }

        Ok(GodotNodeAttrArgs { base, class_name })
    }
}
