use mime::Mime;
use std::collections::HashMap;

/// This type is used to quickly lookup a non-hashed container of mime types by their essence string.
pub(crate) type LookupTable = HashMap<String, Vec<usize>>;

pub(crate) trait LookupTableFromTypes {
    /// Create the lookup table from an iterator of mime types. If `include_stars` is set, the lookup
    /// table will also, for every entry `type/subtype` contain `type/*` and `*/*`.
    fn from_types<'a, I: Iterator<Item = &'a Mime>>(types: I, include_stars: bool) -> Self;
}

fn insert<T>(into: &mut LookupTable, key: T, value: usize)
where
    T: Into<String> + ?Sized,
{
    into.entry(key.into()).or_default().push(value);
}

impl LookupTableFromTypes for LookupTable {
    fn from_types<'a, I: Iterator<Item = &'a Mime>>(types: I, include_stars: bool) -> Self {
        let mut tbl = Self::new();
        for (i, mime) in types.enumerate() {
            if include_stars {
                insert(&mut tbl, "*/*", i);
                insert(&mut tbl, format!("{}/*", mime.type_()), i);
            }
            insert(&mut tbl, mime.essence_str(), i);
        }
        tbl
    }
}
