#![recursion_limit="256"]

extern crate proc_macro;
extern crate url;
extern crate syn;
#[macro_use]
extern crate quote;

mod extractors;
mod extenders;
mod state;
mod helpers;

#[proc_macro_derive(PathExtractor)]
pub fn base_path_extractor(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input(&input.to_string()).unwrap();
    let gen = extractors::base_path(&ast);
    gen.parse().unwrap()
}

#[proc_macro_derive(QueryStringExtractor)]
pub fn base_query_string_extractor(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input(&input.to_string()).unwrap();
    let gen = extractors::base_query_string(&ast);
    gen.parse().unwrap()
}

#[proc_macro_derive(StaticResponseExtender)]
pub fn static_response_extender(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input(&input.to_string()).unwrap();
    let gen = extenders::bad_request_static_response_extender(&ast);
    gen.parse().unwrap()
}

#[proc_macro_derive(StateData)]
pub fn state_data(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input(&input.to_string()).unwrap();
    let gen = state::state_data(&ast);
    gen.parse().unwrap()
}

#[proc_macro_derive(FromState)]
pub fn from_state(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input(&input.to_string()).unwrap();
    let gen = state::from_state(&ast);
    gen.parse().unwrap()
}
