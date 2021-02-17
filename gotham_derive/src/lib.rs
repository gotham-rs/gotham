#![recursion_limit = "256"]
extern crate proc_macro;

mod extenders;
mod new_middleware;
mod state;

#[proc_macro_derive(StaticResponseExtender)]
pub fn static_response_extender(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse(input).unwrap();
    extenders::bad_request_static_response_extender(&ast)
}

#[proc_macro_derive(StateData)]
pub fn state_data(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse(input).unwrap();
    state::state_data(&ast)
}

#[proc_macro_derive(NewMiddleware)]
pub fn new_middleware(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse(input).unwrap();
    new_middleware::new_middleware(&ast)
}
