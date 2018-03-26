use quote;
use syn;

pub(crate) fn state_data(ast: &syn::DeriveInput) -> quote::Tokens {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    quote! {
        impl #impl_generics ::gotham::state::StateData for #name #ty_generics #where_clause {}
    }
}
