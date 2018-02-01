//! This module holds the functions to get and create posts from the DB.

pub mod schema;
pub mod models;


#[macro_use]
extern crate diesel;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use self::models::{Post, NewPost};
use self::schema::posts::dsl::{posts, published};

/// Get the published posts in the DB. Limitted to 5 posts.
pub fn get_posts(conn: &SqliteConnection) -> Vec<Post> {
    posts
        .filter(published.eq(true))
        .limit(5)
        .load::<Post>(conn)
        .expect("Error loading posts")
}

/// Create a new post in the DB.
pub fn create_post<'a>(conn: &SqliteConnection, title: &'a str, body: &'a str) {
    use schema::posts;

    let new_post = NewPost {
        title: title,
        body: body,
        published: true,
    };

    diesel::insert_into(posts::table)
        .values(&new_post)
        .execute(conn)
        .expect("Error saving new post");
}
