use proc_macro;
use syn;

pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input(&input.to_string()).unwrap();

    // This was directly borrowed from the DeepClone example at
    // https://github.com/asajeffrey/deep-clone/blob/master/deep-clone-derive/lib.rs
    // which was instrumental in helping me undertand how to plug this all together.
    let name = &ast.ident;
    let borrowed_lifetime_params = ast.generics
        .lifetimes
        .iter()
        .map(|alpha| quote! { #alpha });
    let borrowed_type_params = ast.generics
        .ty_params
        .iter()
        .map(|ty| quote! { #ty });
    let borrowed_params = borrowed_lifetime_params.chain(borrowed_type_params).collect::<Vec<_>>();
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
    let where_clause_items = type_constraints.chain(where_clause_predicates).collect::<Vec<_>>();
    let where_clause = if where_clause_items.is_empty() {
        quote!{}
    } else {
        quote! { where #(#where_clause_items),* }
    };
    // End of DeepClone borrow, thanks again @asajeffrey.

    let fields = match ast.body {
        syn::Body::Struct(syn::VariantData::Struct(ref body)) => {
            body.iter().filter_map(|field| field.ident.as_ref()).collect::<Vec<_>>()
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

    let mut keys = Vec::new();
    for ident in fields.iter() {
        keys.push(String::from(ident.as_ref()));
    }

    let mut optional_field_labels = Vec::new();
    for ident in optional_fields {
        optional_field_labels.push(String::from(ident.as_ref()))
    }

    let gen = quote! {
        impl #borrowed gotham::state::StateData for #name #borrowed #where_clause {}
        impl #borrowed gotham::http::request_path::RequestPathExtractor for #name #borrowed
             #where_clause
        {
            fn extract(s: &mut gotham::state::State, mut sm: gotham::router::tree::SegmentMapping)
                -> Result<(), String> {
                fn parse<T>(segments: Option<&Vec<&str>>) -> Result<T, String> where T: gotham::http::request_path::FromRequestPath {
                    match segments {
                        Some(segments) => {
                            match T::from_request_path(segments.as_slice()) {
                                Ok(val) => Ok(val),
                                Err(_) => Err(format!("Error converting segments {:?}", segments)),
                            }
                        }
                        None => Err(format!("Error converting segments, none were available")),
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
                let optional_field_labels = [#(#optional_field_labels),*];
                for label in optional_field_labels.iter() {
                    if !sm.contains_key(label) {
                        sm.insert(label, Vec::new());
                    }
                }

                let rp = #name {
                    #(
                        #fields: parse(sm.get(#keys))?,
                     )*
                };

                s.put(rp);
                Ok(())
            }
        }
    };

    gen.parse().unwrap()
}

pub fn is_option(ty: &syn::Ty) -> bool {
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
