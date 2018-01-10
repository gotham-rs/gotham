#![recursion_limit = "256"]

extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

mod extractors;
mod extenders;
mod state;
mod helpers;
mod new_middleware;

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

#[proc_macro_derive(NewMiddleware)]
pub fn new_middleware(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input(&input.to_string()).unwrap();
    let gen = new_middleware::new_middleware(&ast);
    gen.parse().unwrap()
}
