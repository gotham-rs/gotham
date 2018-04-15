use syn;
use quote;

pub(crate) fn new_middleware(ast: &syn::DeriveInput) -> quote::Tokens {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    quote! {
        impl #impl_generics ::gotham::middleware::NewMiddleware for #name #ty_generics
            #where_clause
        {
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
