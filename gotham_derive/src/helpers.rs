use syn;
use quote;

pub fn ty_params<'a>(
    ast: &'a syn::DeriveInput,
    additional_type_constraint: Option<quote::Tokens>,
) -> (&'a syn::Ident, quote::Tokens, quote::Tokens) {
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

    let type_constraints = ast.generics.ty_params.iter().filter_map(|ty| {
        if additional_type_constraint.is_some() {
            Some(quote! { #ty: #additional_type_constraint })
        } else {
            None
        }
    });

    let where_clause_predicates = ast.generics.where_clause.predicates.iter().map(|pred| {
        quote! { #pred }
    });

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
