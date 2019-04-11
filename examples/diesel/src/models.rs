//! Holds the two possible structs that are `Queryable` and
//! `Insertable` in the DB
use diesel::{Insertable, Queryable};
use serde_derive::{Deserialize, Serialize};

use crate::schema::products;

/// Represents a product in the DB.
/// It is `Queryable`
#[derive(Queryable, Serialize, Debug)]
pub struct Product {
    pub id: i32,
    pub title: String,
    pub price: f32,
    pub link: String,
}

/// Represents a new product to insert in the DB.
#[derive(Insertable, Deserialize)]
#[table_name = "products"]
pub struct NewProduct {
    pub title: String,
    pub price: f32,
    pub link: String,
}
