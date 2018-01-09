use syn;
use quote;

use helpers::ty_params;

pub fn new_middleware(ast: &syn::DeriveInput) -> quote::Tokens {
    let (name, borrowed, where_clause) = ty_params(&ast, None);

    quote! {
        impl #borrowed ::gotham::middleware::NewMiddleware for #name #borrowed #where_clause {
            type Instance = Self;

            fn new_middleware(&self) -> ::std::io::Result<Self> {
                // Calling it this way makes the error look like this:
                //
                // | #[derive(NewMiddleware)]
                // |          ^^^^^^^^^^^^^ the trait `std::clone::Clone` is not implemented [...]
                // |
                // = note: required by `std::clone::Clone::clone`
                let new = <Self as Clone>::clone(self);
                Ok(new)
            }
        }
    }
}
