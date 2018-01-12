//! Defines helper functions for the Request query string

use std::collections::HashMap;

use http::{form_url_decode, FormUrlDecoded};

/// Provides a mapping of keys from `Request` query string to their supplied values
#[derive(Debug)]
pub struct QueryStringMapping {
    data: HashMap<String, Vec<FormUrlDecoded>>,
}

impl QueryStringMapping {
    /// Returns a reference for `Request` query string values mapped to the key.
    pub fn get(&self, key: &str) -> Option<&Vec<FormUrlDecoded>> {
        self.data.get(key)
    }

    /// Determines if `Request` query string values exist for the key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Adds an empty value for a key, useful for keys that are considered
    /// optional and haven't been explicitly provided as part of a `Request` query string.
    pub fn add_unmapped_segment(&mut self, key: &str) {
        match form_url_decode(key) {
            Ok(key) => {
                trace!(" unmapped segment {} was added to QueryStringMapping", key);
                self.data.insert(key, Vec::new());
            }
            Err(_) => {
                trace!(
                    " unmapped segment {} was unable to be decoded and will not be added to QueryStringMapping",
                    key
                )
            }
        };
    }
}

/// Splits a query string into pairs and provides a mapping of keys to values.
///
/// For keys which are represented 1..n times in the query string the mapped Vec will be
/// populated with each value provided.
///
/// For keys that are provided but don't have a value associated an empty string will be stored.
///
/// #Examples
///
/// ```rust
/// # extern crate gotham;
/// #
/// # use gotham::http::request::query_string::split;
/// #
/// # pub fn main() {
///       let res = split(Some("key=val&key2=val"));
///       assert_eq!("val", res.get("key").unwrap().first().unwrap().val());
///       assert_eq!("val", res.get("key2").unwrap().first().unwrap().val());
///
///       let res = split(Some("k%65y=val&key=%76al+2"));
///       assert_eq!("val", res.get("key").unwrap().first().unwrap().val());
///       assert_eq!("val 2", res.get("key").unwrap().last().unwrap().val());
///
///       let res = split(Some("key=val&key2="));
///       assert_eq!("val", res.get("key").unwrap().first().unwrap().val());
///       assert_eq!("", res.get("key2").unwrap().first().unwrap().val());
/// # }
/// ```
pub fn split<'r>(query: Option<&'r str>) -> QueryStringMapping {
    match query {
        Some(query) => {
            let pairs = query.split("&").filter(|pair| pair.contains("="));
            let data = pairs.fold(HashMap::new(), |mut acc, p| {
                let mut sp = p.split("=");
                let (k, v) = (sp.next().unwrap(), sp.next().unwrap());
                match form_url_decode(k) {
                    Ok(k) => {
                        let vec = acc.entry(k).or_insert(Vec::new());
                        match FormUrlDecoded::new(v) {
                            Some(dv) => vec.push(dv),
                            None => (),
                        }
                    }
                    Err(_) => (),
                };
                acc
            });

            QueryStringMapping { data }
        }
        None => QueryStringMapping {
            data: HashMap::new(),
        },
    }
}
