use syn;
use quote;

pub fn base_path(_ast: &syn::DeriveInput) -> quote::Tokens {
    quote! {
        compile_error!("#[derive(PathExtractor)] is no longer supported - please switch to \
                        #[derive(Deserialize)]. The `StateData` and `StaticResponseExtender` \
                        derives are still required.");
    }
}

pub fn base_query_string(_ast: &syn::DeriveInput) -> quote::Tokens {
    quote! {
        compile_error!("#[derive(QueryStringExtractor)] is no longer supported - please switch to \
                        #[derive(Deserialize)]. The `StateData` and `StaticResponseExtender` \
                        derives are still required.");
    }
}
