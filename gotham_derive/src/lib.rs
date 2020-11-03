#![recursion_limit = "256"]
extern crate proc_macro;

use proc_macro::TokenStream;
use syn::parse_macro_input;

mod extenders;
mod new_middleware;
mod path_template;
mod state;

#[proc_macro_derive(StaticResponseExtender)]
pub fn static_response_extender(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input);
    extenders::bad_request_static_response_extender(&ast)
}

#[proc_macro_derive(StateData)]
pub fn state_data(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input);
    state::state_data(&ast)
}

#[proc_macro_derive(NewMiddleware)]
pub fn new_middleware(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input);
    new_middleware::new_middleware(&ast)
}

#[proc_macro_derive(PathTemplate, attributes(path_template))]
pub fn path_template(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input);
    path_template::path_template(&ast)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
