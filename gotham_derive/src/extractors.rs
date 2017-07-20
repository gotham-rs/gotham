use syn;
use quote;

use helpers::{ty_params, ty_fields};

pub fn base_path(ast: &syn::DeriveInput) -> quote::Tokens {
    let (name, borrowed, where_clause) = ty_params(&ast);
    let (fields, optional_fields) = ty_fields(&ast);
    let ofl = optional_field_labels(optional_fields);
    let ofl_len = ofl.len();
    let keys = field_names(&fields);

    quote! {
        impl #borrowed gotham::state::StateData for #name #borrowed #where_clause {}
        impl #borrowed gotham::router::request::path::RequestPathExtractor for #name #borrowed
             #where_clause
        {
            fn extract(s: &mut gotham::state::State, mut sm: gotham::router::tree::SegmentMapping)
                -> Result<(), String>
            {
                fn parse<T>(s: &gotham::state::State, segments: Option<&Vec<&gotham::http::PercentDecoded>>) -> Result<T, String>
                    where T: gotham::router::request::path::FromRequestPath
                {
                    match segments {
                        Some(segments) => {
                            match T::from_request_path(segments.as_slice()) {
                                Ok(val) => {
                                    trace!("[{}] extracted request path segments", gotham::state::request_id(s));
                                    Ok(val)
                                }
                                Err(_) => {
                                    error!("[{}] unrecoverable error converting request path", gotham::state::request_id(s));
                                    Err(String::from("unrecoverable error converting request path"))
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

pub fn base_query_string(ast: &syn::DeriveInput) -> quote::Tokens {
    let (name, borrowed, where_clause) = ty_params(&ast);
    let (fields, optional_fields) = ty_fields(&ast);
    let ofl = optional_field_labels(optional_fields);
    let ofl_len = ofl.len();
    let keys = field_names(&fields);
    let keys2 = keys.clone();

    quote! {
        impl #borrowed gotham::state::StateData for #name #borrowed #where_clause {}
        impl #borrowed gotham::router::request::query_string::QueryStringExtractor for #name #borrowed
             #where_clause
        {
            fn extract(s: &mut gotham::state::State, query: Option<&str>) -> Result<(), String> {
                fn parse<T>(s: &gotham::state::State, key: &str, values: Option<&Vec<gotham::http::FormUrlDecoded>>) -> Result<T, String>
                    where T: gotham::router::request::query_string::FromQueryString
                {
                    match values {
                        Some(values) => {
                            match T::from_query_string(key, values.as_slice()) {
                                Ok(val) => {
                                    trace!("[{}] extracted query string values", gotham::state::request_id(&s));
                                    Ok(val)
                                }
                                Err(_) => {
                                    error!("[{}] unrecoverable error converting query string", gotham::state::request_id(&s));
                                    Err(String::from("unrecoverable error converting query string"))
                                }
                            }
                        }
                        None => Err(format!("error converting query string value `{}`", key))
                    }
                }

                let mut qsm = gotham::http::request::query_string::split(query);
                trace!("[{}] query string mappings recieved from client: {:?}", gotham::state::request_id(s), qsm);

                // Add an empty Vec for Optional segments that have not been provided.
                //
                // Ideally `optional_fields` would be a const but this doesn't yet seem to be
                // possible when using the `quote` crate as we are here.
                let ofl:[&str; #ofl_len] = [#(#ofl), *];
                for label in ofl.iter() {
                    if !qsm.contains_key(label) {
                        trace!(" adding unmapped value: {:?}", label);
                        qsm.add_unmapped_segment(label);
                    }
                }

                trace!("[{}] query string mappings to be parsed: {:?}", gotham::state::request_id(s), qsm);

                let qss = #name {
                    #(
                        #fields: parse(s, #keys, qsm.get(#keys2))?,
                     )*
                };
                trace!("[{}] query string struct created and stored in state", gotham::state::request_id(s));

                s.put(qss);
                Ok(())
            }
        }
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
