#![recursion_limit="128"]

extern crate proc_macro;
extern crate url;
extern crate syn;
#[macro_use]
extern crate quote;

mod extractors;

#[proc_macro_derive(RequestPathExtractor)]
pub fn request_path_extractor(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    extractors::request_path(input)
}

#[proc_macro_derive(QueryStringExtractor)]
pub fn query_string_extractor(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    extractors::query_string(input)
}
