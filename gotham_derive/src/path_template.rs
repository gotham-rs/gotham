use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{DeriveInput, Error, Lit, LitStr, Meta, Result};

pub fn path_template(ast: &DeriveInput) -> Result<TokenStream> {
    let ty_name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    // parse attributes
    let mut path_template: Option<LitStr> = None;
    for attr in ast.attrs.iter() {
        let attr = match attr.parse_meta() {
            Ok(Meta::NameValue(nv)) => nv,
            _ => continue,
        };
        if attr
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .as_deref()
            != Some("path_template")
        {
            continue;
        }

        path_template = Some(match attr.lit {
            Lit::Str(str) => str,
            lit => {
                return Err(Error::new(
                    lit.span(),
                    "Expected string literal, e.g. #[path_template = \"/foo/:bar\"]",
                ))
            }
        });
    }
    let path_template = path_template.ok_or_else(|| {
        Error::new(
            Span::call_site(),
            "#[derive(PathTemplate)] requires a #[path_template = \"/foo/:bar\"] attribute",
        )
    })?;

    // TODO verify that the path parameters match the types fields

    // generate the implementation
    Ok(quote! {
        impl #impl_generics ::gotham::router::builder::PathTemplate for #ty_name #ty_generics #where_clause {
            fn path_template() -> ::std::borrow::Cow<'static, str> {
                let path_template: &'static str = #path_template;
                path_template.into()
            }
        }
    })
}
