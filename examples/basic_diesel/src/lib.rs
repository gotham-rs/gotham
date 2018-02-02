//! This module holds the functions to get and create posts from the DB.

pub mod schema;
pub mod models;


#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde_derive;
extern crate serde;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use self::models::{Post, NewPost};
use self::schema::posts::dsl::{posts, published};


/// Get the published posts in the DB. Limitted to 5 posts.
pub fn get_posts(conn: &SqliteConnection) -> Vec<Post> {
    // Check the result of the transaction.
    // If there's an error, try to create the DB and return an empty `Vec`
    match posts.filter(published.eq(true)).limit(5).load::<Post>(conn) {
        Ok(post) => post,
        Err(e) => {
            println!("Problem encountered: {}", e);
            create_table(conn);
            vec![]
        }
    }
}

/// Create a new post in the DB.
pub fn create_post<'a>(
    conn: &SqliteConnection,
    title: &'a str,
    body: &'a str,
) -> QueryResult<usize> {
    use schema::posts;

    let new_post = NewPost {
        title: title,
        body: body,
        published: true,
    };

    // Check the result of the transaction.
    // If there's an error, try to create the DB an re-run the transaction
    match diesel::insert_into(posts::table)
        .values(&new_post)
        .execute(conn) {
        Ok(i) => Ok(i),
        Err(e) => {
            println!("Problem encountered: {}", e);
            create_table(conn);
            diesel::insert_into(posts::table)
                .values(&new_post)
                .execute(conn)
        }
    }
}

/// Create the table if needed.
fn create_table(conn: &SqliteConnection) {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS posts (
        id INTEGER PRIMARY KEY,
        title VARCHAR NOT NULL,
        body TEXT NOT NULL,
        published BOOLEAN NOT NULL DEFAULT 'f'
        )"
    ).expect("Could not create table");
}
