use proc_macro;
use syn;
use quote;

pub fn request_path(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input(&input.to_string()).unwrap();
    let (name, borrowed, where_clause) = ty_params(&ast);
    let (fields, optional_fields) = ty_fields(&ast);
    let ofl = optional_field_labels(optional_fields);
    let keys = field_names(&fields);

    let gen = quote! {
        impl #borrowed gotham::state::StateData for #name #borrowed #where_clause {}
        impl #borrowed gotham::http::request_path::RequestPathExtractor for #name #borrowed
             #where_clause
        {
            fn extract(s: &mut gotham::state::State, mut sm: gotham::router::tree::SegmentMapping)
                -> Result<(), String>
            {
                fn parse<T>(s: &gotham::state::State, segments: Option<&Vec<&gotham::http::PercentDecoded>>) -> Result<T, String>
                    where T: gotham::http::request_path::FromRequestPath
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
                let ofl = [#(#ofl),*];
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
    };

    gen.parse().unwrap()
}

pub fn query_string(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input(&input.to_string()).unwrap();
    let (name, borrowed, where_clause) = ty_params(&ast);
    let (fields, optional_fields) = ty_fields(&ast);
    let ofl = optional_field_labels(optional_fields);
    let keys = field_names(&fields);
    let keys2 = keys.clone();

    let gen = quote! {
        impl #borrowed gotham::state::StateData for #name #borrowed #where_clause {}
        impl #borrowed gotham::http::query_string::QueryStringExtractor for #name #borrowed
             #where_clause
        {
            fn extract(s: &mut gotham::state::State, query: Option<&str>) -> Result<(), String> {
                fn parse<T>(s: &gotham::state::State, key: &str, values: Option<&Vec<gotham::http::FormUrlDecoded>>) -> Result<T, String>
                    where T: gotham::http::query_string::FromQueryString
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
                        None => Err(format!("error converting query string value `{}`", key)),
                    }
                }

                let mut qsm = gotham::http::query_string::split(query);
                trace!("[{}] query string mappings recieved from client: {:?}", gotham::state::request_id(s), qsm);

                // Add an empty Vec for Optional segments that have not been provided.
                //
                // Ideally `optional_fields` would be a const but this doesn't yet seem to be
                // possible when using the `quote` crate as we are here.
                let ofl = [#(#ofl),*];
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
    };

    gen.parse().unwrap()
}

fn ty_params<'a>(ast: &'a syn::DeriveInput) -> (&'a syn::Ident, quote::Tokens, quote::Tokens) {
    // This was directly borrowed from the DeepClone example at
    // https://github.com/asajeffrey/deep-clone/blob/master/deep-clone-derive/lib.rs
    // which was instrumental in helping me undertand how to plug this all together.
    let name = &ast.ident;
    let borrowed_lifetime_params = ast.generics.lifetimes.iter().map(|alpha| quote! { #alpha });
    let borrowed_type_params = ast.generics.ty_params.iter().map(|ty| quote! { #ty });
    let borrowed_params = borrowed_lifetime_params
        .chain(borrowed_type_params)
        .collect::<Vec<_>>();
    let borrowed = if borrowed_params.is_empty() {
        quote!{}
    } else {
        quote! { < #(#borrowed_params),* > }
    };

    let type_constraints = ast.generics
        .ty_params
        .iter()
        .map(|ty| quote! { #ty: RequestPathExtractor });
    let where_clause_predicates = ast.generics
        .where_clause
        .predicates
        .iter()
        .map(|pred| quote! { #pred });
    let where_clause_items = type_constraints
        .chain(where_clause_predicates)
        .collect::<Vec<_>>();
    let where_clause = if where_clause_items.is_empty() {
        quote!{}
    } else {
        quote! { where #(#where_clause_items),* }
    };
    // End of DeepClone borrow, thanks again @asajeffrey.

    (name, borrowed, where_clause)
}

fn ty_fields<'a>(ast: &'a syn::DeriveInput) -> (Vec<&syn::Ident>, Vec<&syn::Ident>) {
    let fields = match ast.body {
        syn::Body::Struct(syn::VariantData::Struct(ref body)) => {
            body.iter()
                .filter_map(|field| field.ident.as_ref())
                .collect::<Vec<_>>()
        }
        _ => panic!("Not implemented for tuple or unit like structs"),
    };

    let optional_fields = match ast.body {
        syn::Body::Struct(syn::VariantData::Struct(ref body)) => {
            body.iter()
                .filter_map(|field| if is_option(&field.ty) {
                                field.ident.as_ref()
                            } else {
                                None
                            })
                .collect::<Vec<_>>()
        }
        _ => panic!("Not implemented for tuple or unit like structs"),
    };

    (fields, optional_fields)
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

fn is_option(ty: &syn::Ty) -> bool {
    match *ty {
        syn::Ty::Path(_, ref p) => {
            match p.segments.first() {
                Some(segment) => segment.ident == syn::Ident::from("Option"),
                None => false,
            }
        }
        _ => false,
    }
}
