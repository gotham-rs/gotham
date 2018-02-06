//! Holds the two possible structs that are `Queryable` and
//! `Insertable` in the DB

use super::schema::products;

/// Represents a product in the DB.
/// It is `Queryable`
#[derive(Queryable, Serialize, Debug)]
pub struct Product {
    pub id: Option<i32>,
    pub title: String,
    pub price: f32,
    pub link: String,
}

/// Represents a new product to insert in the DB.
#[derive(Insertable, Deserialize)]
#[table_name = "products"]
pub struct NewProduct<'a> {
    pub title: &'a str,
    pub price: f32,
    pub link: String,
}
