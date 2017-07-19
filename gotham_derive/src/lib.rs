#![recursion_limit="128"]

extern crate proc_macro;
extern crate url;
extern crate syn;
#[macro_use]
extern crate quote;

mod extractors;
mod helpers;

use helpers::ty_params;

#[proc_macro_derive(RequestPathExtractor)]
pub fn request_path_extractor(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    extractors::request_path(input)
}

#[proc_macro_derive(QueryStringExtractor)]
pub fn query_string_extractor(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    extractors::query_string(input)
}

#[proc_macro_derive(StateData)]
pub fn state_data(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input(&input.to_string()).unwrap();
    let (name, borrowed, where_clause) = ty_params(&ast);
    let gen = quote! {
        impl #borrowed gotham::state::StateData for #name #borrowed #where_clause {}
    };

    gen.parse().unwrap()
}
