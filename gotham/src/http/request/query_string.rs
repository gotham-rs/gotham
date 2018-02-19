//! Defines helper functions for the Request query string

use std::collections::HashMap;

use http::{form_url_decode, FormUrlDecoded};

/// Provides a mapping of keys from `Request` query string to their supplied values
pub(crate) type QueryStringMapping = HashMap<String, Vec<FormUrlDecoded>>;

/// Splits a query string into pairs and provides a mapping of keys to values.
///
/// For keys which are represented 1..n times in the query string the mapped Vec will be
/// populated with each value provided.
///
/// For keys that are provided but don't have a value associated an empty string will be stored.
pub(crate) fn split<'r>(query: Option<&'r str>) -> QueryStringMapping {
    match query {
        Some(query) => {
            let pairs = query.split("&").filter(|pair| pair.contains("="));

            let mut query_string_mapping = QueryStringMapping::new();

            for p in pairs {
                let mut sp = p.split("=");
                let (k, v) = (sp.next().unwrap(), sp.next().unwrap());
                match form_url_decode(k) {
                    Ok(k) => {
                        let vec = query_string_mapping.entry(k).or_insert(Vec::new());
                        match FormUrlDecoded::new(v) {
                            Some(dv) => vec.push(dv),
                            None => (),
                        }
                    }
                    Err(_) => (),
                };
            }

            query_string_mapping
        }
        None => QueryStringMapping::new(),
    }
}
