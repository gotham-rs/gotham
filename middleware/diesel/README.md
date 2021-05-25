# Gotham Diesel Middleware

The gotham diesel middleware provides offers a convenient API for interacting with Diesel from Gotham.

## Usage:
This middleware introduces a Repo struct, which is used as a layer between Diesel and Gotham to ensure that database interaction can be easily chained alongside other asynchronous operations. This structure is fairly straightfoward and offers an easy way to interact with Diesel from inside Gotham:
```rust
// create a new repo, in this case just using a SQLite setup
let repo: Repo<SqliteConnection> = Repo::new("products.db");

// create a middleware pipeline from our middleware
let pipeline = single_middleware(DieselMiddleware::new(repo));

// construct a basic chain from our pipeline
let (chain, pipelines) = single_pipeline(pipeline);

// build a router with the chain & pipeline
gotham::start("127.0.0.1:7878", build_router(chain, pipelines, |route| {
    route.get("/").to(say_hello);
}))
```
From there you gain simple access to Repo on the request state, just like when using other middlewares. You can then use the Repo to execute database calls:
```rust
// borrow the repo from the state
let repo = Repo::borrow_from(&state);

// execute database calls
repo.run(move |conn| {
    diesel::insert_into(products::table)
        .values(&product)
        .execute(&conn)
})
```
`repo.run` returns a Future, allowing you to seamlessly sprinkle your database calls amongst other asynchronous handler code. The `Repo` type manages the synchronous calls of the underlying connections using `tokio::task::spawn_blocking`, which allows blocking operations to run without blocking the tokio reactor. Although not true async, this allows multiple concurrent database requests to be handled, with a default of 100 concurrent blocking operations. For further details see [tokio::task::spawn_blocking documentation](https://docs.rs/tokio/1/tokio/task/fn.spawn_blocking.html).

For a complete example, see the [example in the main repository](https://github.com/gotham-rs/gotham/tree/master/examples/diesel).

## Configuration
To customize aspects of the connection pool, you can construct a repo from an `r2d2::Builder`, setting any attributes available on there:
```rust
let repo = Repo::from_pool_builder(database_url,
    Pool::builder()
        .connection_timeout(Duration::from_secs(120))
        .max_size(100)
```

## Isolated test transactions
When used in tests, the middleware can use isolated test transactions to allow
tests to run in parallel. In test transactions, queries from separate connections do not interfere with each other and are rolled back when the connection is dropped at the end of each test.
```
#[test]
fn do_something() {
    let repo = Repo::with_test_transactions(DATABASE_URL);
    // Run some test code that accesses the repo.
    // This test will be isolated, and at the end the transaction rolled back.
}
```
See full example in the main repository linked above for more details.
