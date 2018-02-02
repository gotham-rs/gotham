//! Holds the two possible structs that are `Queryable` and
//! `Insertable` in the DB

use super::schema::posts;

/// Represents a post in the DB.
/// It is `Queryable`
#[derive(Queryable, Serialize, Debug)]
pub struct Post {
    pub id: Option<i32>,
    pub title: String,
    pub body: String,
    pub published: bool,
}

/// Represents a new post to insert in the DB.
#[derive(Insertable, Deserialize)]
#[table_name = "posts"]
pub struct NewPost<'a> {
    pub title: &'a str,
    pub body: &'a str,
    pub published: bool,
}
