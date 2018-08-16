use proc_macro;
use syn;

pub(crate) fn bad_request_static_response_extender(
    ast: &syn::DeriveInput,
) -> proc_macro::TokenStream {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics ::gotham::router::response::extender::StaticResponseExtender for #name
            #ty_generics #where_clause
        {
            type ResBody = ::hyper::body::Body;

            fn extend(state: &mut ::gotham::state::State, res: &mut ::hyper::Response<Self::ResBody>) {
                res.headers_mut().insert(::gotham::helpers::http::header::X_REQUEST_ID,
                                         ::gotham::state::request_id(state).parse().unwrap());
                *res.status_mut() = ::hyper::StatusCode::BAD_REQUEST;
            }
        }
    };

    expanded.into()
}
