# Middleware for Diesel

[Diesel](https://diesel.rs) is a safe, extensible ORM and query builder
for Rust. Diesel is the most productive way to interact with databases
in Rust because of its safe and composable abstractions over queries.

This middleware provides a convenient mechanism to setup a pool of
connections for Postgres, MySQL or Sqlite database and provide one of
those connections, per Request, to a Gotham application via `state`.

**This middleware is under active development**

n.b. Diesel does not yet natively support async.
The API here will initially use CpuPool to make life easier for Gotham
apps but will use native Diesel async support once
[available](https://github.com/diesel-rs/diesel/issues/399).

## License

Licensed under your option of:

* [MIT License](../LICENSE-MIT)
* [Apache License, Version 2.0](../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../../CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
