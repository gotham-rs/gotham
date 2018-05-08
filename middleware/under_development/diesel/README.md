# Middleware for Diesel

[Diesel](https://diesel.rs) is a safe, extensible ORM and query builder for
Rust. Diesel is the most productive way to interact with databases in Rust
because of its safe and composable abstractions over queries.

This middleware provides an interface for running Diesel queries in a Gotham
application.

As Diesel [only provides support for synchronous][async-diesel] database access,
we use the [`WorkersMiddleware`][workers] to execute queries in the "background"
and prevent applications from blocking the event loop.

[workers]: https://github.com/gotham-rs/gotham/tree/master/middleware/under_development/workers
[async-diesel]: https://github.com/diesel-rs/diesel/issues/399

**This middleware is under active development**

## Performance caveats

Executing synchronous logic in a background thread is not intended to be a
perfect replacement for an asynchronous database library.

We expect the performance characteristics of an application with an equal number
of worker threads and pooled database connections to be comparable to an
asynchronous database library with the same number of pooled database
connections. The asynchronous database library may be able to maintain a larger
connection pool without reaching limits imposed by the kernel. (For example, a
hard limit on the number of threads a process may create.)

It's important to understand the trade-off that this middleware makes and decide
whether it's appropriate for your application and the amount of load you expect
to receive.

## License

Licensed under your option of:

* [MIT License](../LICENSE-MIT)
* [Apache License, Version 2.0](../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Code of conduct](../../CODE_OF_CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
