//! This module holds the functions to get and create products from the DB.

pub mod schema;
pub mod models;

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use self::models::{NewProduct, Product};
use self::schema::products::dsl::products;

embed_migrations!();

/// Get the published products in the DB. Limitted to 5 products.
pub fn get_products(conn: &SqliteConnection) -> Vec<Product> {
    // Run the migrations to be sure that the `products` table is present
    let _result = embedded_migrations::run(conn);

    products.limit(5).load::<Product>(conn).unwrap()
}

/// Create a new product in the DB.
pub fn create_product<'a>(
    conn: &SqliteConnection,
    title: &'a str,
    price: f32,
    link: String,
) -> QueryResult<usize> {
    use schema::products;
    // Run the migrations to be sure that the `products` table is present
    let _result = embedded_migrations::run(conn);

    let new_product = NewProduct {
        title: title,
        price: price,
        link: link,
    };

    // Insert the `NewProduct` in the DB
    diesel::insert_into(products::table)
        .values(&new_product)
        .execute(conn)
}
