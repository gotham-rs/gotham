use syn;
use quote;

use helpers::{ty_fields, ty_params};

pub fn base_path(ast: &syn::DeriveInput) -> quote::Tokens {
    let (name, borrowed, where_clause) = ty_params(&ast, None);
    let (fields, optional_fields) = ty_fields(&ast);
    let ofl = optional_field_labels(optional_fields);
    let ofl_len = ofl.len();
    let keys = field_names(&fields);

    let struct_name_token = quote!{#name};
    let struct_name = struct_name_token.as_str();

    quote! {
        impl #borrowed ::gotham::router::request::path::PathExtractor for #name #borrowed
             #where_clause
        {
            fn extract(
                s: &mut ::gotham::state::State,
                mut sm: ::gotham::router::tree::SegmentMapping
            ) -> Result<(), String> {
                fn parse<T>(
                    s: &::gotham::state::State,
                    segments: Option<&Vec<&::gotham::http::PercentDecoded>>
                ) -> Result<T, String>
                where
                    T: ::gotham::router::request::path::FromRequestPath,
                {
                    let struct_name = #struct_name;
                    match segments {
                        Some(segments) => {
                            match T::from_request_path(segments.as_slice()) {
                                Ok(val) => {
                                    Ok(val)
                                }
                                Err(_) => {
                                    Err(format!("[{}] unrecoverable error converting request path, into {}",
                                                ::gotham::state::request_id(s), struct_name))
                                }
                            }
                        }
                        None => Err(String::from("Error converting Request path values")),
                    }
                }

                // Add an empty Vec for Optional segments that have not been provided.
                //
                // This essentially indicates that a single Struct is being used for multiple
                // Request paths and ending at different Handlers.
                //
                // Not a best practice approach but worth supporting.
                //
                // Ideally `optional_fields` would be a const but this doesn't yet seem to be
                // possible when using the `quote` crate as we are here.
                let ofl:[&str; #ofl_len] = [#(#ofl), *];
                for label in ofl.iter() {
                    if !sm.contains_key(label) {
                        sm.add_unmapped_segment(label);
                    }
                }

                let rp = #name {
                    #(
                        #fields: parse(s, sm.get(#keys))?,
                     )*
                };

                s.put(rp);
                Ok(())
            }
        }
    }
}

pub fn base_query_string(_ast: &syn::DeriveInput) -> quote::Tokens {
    quote! {
        compile_error!("#[derive(QueryStringExtractor)] is no longer supported - please switch to \
                        #[derive(Deserialize)]. The `StateData` and `StaticResponseExtender` \
                        derives are still required.");
    }
}

fn optional_field_labels<'a>(optional_fields: Vec<&'a syn::Ident>) -> Vec<&'a str> {
    let mut ofl = Vec::new();
    for ident in optional_fields {
        ofl.push(ident.as_ref())
    }
    ofl
}

fn field_names<'a>(fields: &'a Vec<&'a syn::Ident>) -> Vec<String> {
    let mut keys = Vec::new();
    for ident in fields.iter() {
        keys.push(String::from(ident.as_ref()));
    }
    keys
}
