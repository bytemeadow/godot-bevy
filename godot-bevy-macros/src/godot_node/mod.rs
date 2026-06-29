pub fn derive_godot_node_component(
    input: syn::DeriveInput,
) -> syn::Result<proc_macro2::TokenStream> {
    let plan = crate::bevy_attr::parse_component_first(&input)?;
    Ok(crate::emit::emit(&plan, &input))
}

pub fn derive_bevy_components(input: syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let plan = crate::bevy_attr::parse_godot_first(&input)?;
    Ok(crate::emit::emit(&plan, &input))
}
