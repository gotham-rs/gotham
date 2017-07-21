use syn;
use quote;

use helpers::ty_params;

pub fn state_data(ast: &syn::DeriveInput) -> quote::Tokens {
    let (name, borrowed, where_clause) = ty_params(&ast);

    quote! {
        impl #borrowed gotham::state::StateData for #name #borrowed #where_clause {}
    }
}

pub fn from_state(ast: &syn::DeriveInput) -> quote::Tokens {
    let (name, borrowed, where_clause) = ty_params(&ast);

    let struct_name_token = quote!{#name};
    let struct_name = struct_name_token.as_str();

    quote! {
        impl #borrowed gotham::state::FromState<Self> for #name #borrowed #where_clause {
            fn take_from(s: &mut gotham::state::State) -> Self {
                s.take::<Self>()
                 .unwrap_or_else(|| {
                     let struct_name = #struct_name;
                     panic!("[{}] [take] {} is not stored in State",
                            gotham::state::request_id(s),
                            struct_name)
                 })
            }

            fn borrow_from(s: &gotham::state::State) -> &Self {
                s.borrow::<Self>()
                 .unwrap_or_else(|| {
                     let struct_name = #struct_name;
                     panic!("[{}] [borrow] {} is not stored in State",
                            gotham::state::request_id(s),
                            struct_name)
                 })
            }

            fn borrow_mut_from(s: &mut gotham::state::State) -> &mut Self {
                let req_id = String::from(gotham::state::request_id(s));
                s.borrow_mut::<Self>()
                 .unwrap_or_else(|| {
                     let struct_name = #struct_name;
                     panic!("[{}] [borrow_mut] {} is not stored in State",
                            req_id,
                            struct_name)
                 })
            }
        }
    }
}
