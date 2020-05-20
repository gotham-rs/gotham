use itertools::Itertools;
use mime::Mime;
use std::collections::HashMap;

/// This type is used to quickly lookup a non-hashed container of mime types by their essence string.
pub type LookupTable = HashMap<String, Vec<usize>>;

pub trait LookupTableFromTypes {
    /// Create the lookup table from an iterator of mime types. If `include_stars` is set, the lookup
    /// table will also, for every entry `type/subtype` contain `type/*` and `*/*`.
    fn from_types<'a, I: Iterator<Item = &'a Mime>>(types: I, include_stars: bool) -> Self;
}

impl LookupTableFromTypes for LookupTable {
    fn from_types<'a, I: Iterator<Item = &'a Mime>>(types: I, include_stars: bool) -> Self {
        if include_stars {
            types
                .enumerate()
                .flat_map(|(i, mime)| {
                    vec![
                        ("*/*".to_owned(), i),
                        (format!("{}/*", mime.type_()), i),
                        (mime.essence_str().to_owned(), i),
                    ]
                    .into_iter()
                })
                .into_group_map()
        } else {
            types
                .enumerate()
                .map(|(i, mime)| (mime.essence_str().to_owned(), i))
                .into_group_map()
        }
    }
}
