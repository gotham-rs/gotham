//! This module holds the functions to get and create posts from the DB.

pub mod schema;
pub mod models;


#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate serde_derive;
extern crate serde;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use self::models::{Post, NewPost};
use self::schema::posts::dsl::{posts, published};

embed_migrations!();

/// Get the published posts in the DB. Limitted to 5 posts.
pub fn get_posts(conn: &SqliteConnection) -> Vec<Post> {
    // Run the migrations to be sure that the `posts` table is present
    let _result = embedded_migrations::run(conn);

    posts
        .filter(published.eq(true))
        .limit(5)
        .load::<Post>(conn)
        .unwrap()
}

/// Create a new post in the DB.
pub fn create_post<'a>(
    conn: &SqliteConnection,
    title: &'a str,
    body: &'a str,
) -> QueryResult<usize> {
    use schema::posts;
    // Run the migrations to be sure that the `posts` table is present
    let _result = embedded_migrations::run(conn);

    let new_post = NewPost {
        title: title,
        body: body,
        published: true,
    };

    // Insert the `NewPost` in the DB 
    diesel::insert_into(posts::table)
        .values(&new_post)
        .execute(conn)
}

