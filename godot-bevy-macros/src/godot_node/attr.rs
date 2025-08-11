use syn::parse::{Parse, ParseStream};

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
