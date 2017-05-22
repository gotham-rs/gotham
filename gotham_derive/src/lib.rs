#![recursion_limit="128"]

extern crate proc_macro;
extern crate url;
extern crate syn;
#[macro_use]
extern crate quote;

mod request_path_extractor;

#[proc_macro_derive(RequestPathExtractor)]
pub fn request_path_extractor(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    request_path_extractor::derive(input)
}
