// @generated automatically by Diesel CLI.

diesel::table! {
    products (id) {
        id -> Integer,
        title -> Text,
        price -> Float,
        link -> Text,
    }
}
