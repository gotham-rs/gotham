use proc_macro;
use syn;

pub(crate) fn base_path(_ast: &syn::DeriveInput) -> proc_macro::TokenStream {
    let expanded = quote! {
        compile_error!("#[derive(PathExtractor)] is no longer supported - please switch to \
                        #[derive(Deserialize)]. The `StateData` and `StaticResponseExtender` \
                        derives are still required.");
    };
    expanded.into()
}

pub(crate) fn base_query_string(_ast: &syn::DeriveInput) -> proc_macro::TokenStream {
    let expanded = quote! {
        compile_error!("#[derive(QueryStringExtractor)] is no longer supported - please switch to \
                        #[derive(Deserialize)]. The `StateData` and `StaticResponseExtender` \
                        derives are still required.");
    };
    expanded.into()
}
