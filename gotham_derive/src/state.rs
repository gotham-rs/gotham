use syn;
use quote;

use helpers::ty_params;

pub fn state_data(ast: &syn::DeriveInput) -> quote::Tokens {
    let (name, borrowed, where_clause) = ty_params(&ast, None);

    quote! {
        impl #borrowed ::gotham::state::StateData for #name #borrowed #where_clause {}
    }
}
