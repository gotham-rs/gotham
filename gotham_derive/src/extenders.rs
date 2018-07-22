use quote;
use syn;

pub(crate) fn bad_request_static_response_extender(ast: &syn::DeriveInput) -> quote::Tokens {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    quote! {
        impl #impl_generics ::gotham::router::response::extender::StaticResponseExtender for #name
            #ty_generics #where_clause
        {
            type ResBody = ::hyper::body::Body;

            fn extend(state: &mut ::gotham::state::State, res: &mut ::hyper::Response<Self::ResBody>) {
                ::gotham::helpers::http::response::extend_response(
                    state,
                    res,
                    ::hyper::StatusCode::BadRequest,
                    None,
                );
            }
        }
    }
}
