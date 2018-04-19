//! Defines helper functions for the Request query string

use std::collections::HashMap;

use protocol::{form_url_decode, FormUrlDecoded};

/// Provides a mapping of keys from `Request` query string to their supplied values
pub(crate) type QueryStringMapping = HashMap<String, Vec<FormUrlDecoded>>;

/// Splits a query string into pairs and provides a mapping of keys to values.
///
/// For keys which are represented 1..n times in the query string the mapped `Vec` will be
/// populated with each value provided.
///
/// Keys that are provided but with no value associated are skipped.
pub(crate) fn split<'r>(query: Option<&'r str>) -> QueryStringMapping {
    match query {
        Some(query) => {
            let pairs = query.split(is_separator).filter(|pair| pair.contains("="));

            let mut query_string_mapping = QueryStringMapping::new();

            for p in pairs {
                let mut sp = p.splitn(2, '=');
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

fn is_separator(c: char) -> bool {
    c == '&' || c == ';'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_pairs<'a>(qsm: &'a QueryStringMapping) -> Vec<(&'a str, Vec<&'a str>)> {
        let mut pairs: Vec<(&str, Vec<&str>)> = qsm.iter()
            .map(|(k, v)| {
                let mut values: Vec<&str> = v.iter().map(|s| s.as_ref()).collect();
                values.sort();

                (k.as_str(), values)
            })
            .collect();

        pairs.sort_by(|&(ref a, ref _a_val), &(ref b, ref _b_val)| a.cmp(b));
        pairs
    }

    #[test]
    fn query_string_mapping_tests() {
        let qsm = split(Some("a=b&c=d&e=f"));
        assert_eq!(
            to_pairs(&qsm),
            vec![("a", vec!["b"]), ("c", vec!["d"]), ("e", vec!["f"])],
        );

        let qsm = split(Some("a=b&a=d&e=f"));
        assert_eq!(
            to_pairs(&qsm),
            vec![("a", vec!["b", "d"]), ("e", vec!["f"])],
        );

        let qsm = split(Some("a&b"));
        assert_eq!(to_pairs(&qsm), vec![],);

        let qsm = split(Some("a=b;c=d&e=f"));
        assert_eq!(
            to_pairs(&qsm),
            vec![("a", vec!["b"]), ("c", vec!["d"]), ("e", vec!["f"])],
        );

        let qsm = split(Some("a=b=c&d=e"));
        assert_eq!(to_pairs(&qsm), vec![("a", vec!["b=c"]), ("d", vec!["e"])],);
    }
}
