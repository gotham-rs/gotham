# Examples

A collection of crates that provide examples for developing web applications
with the Gotham web framework.

All crates include test cases that prove correct behaviour and serve as an
example of how to test your applications.

## Rust

These examples assume a familiarity with the Rust programming language. If
you're new to Rust itself the following resources will assist you to get
started:

1. [The Rust book](https://doc.rust-lang.org/book/second-edition/)
1. [Programming Rust](http://shop.oreilly.com/product/0636920040385.do)
1. [Exercisms for Rust](http://exercism.io/languages/rust/about)
1. [The Rust users community](https://users.rust-lang.org)
1. [Rust subreddit](https://reddit.com/r/rust)

## Ordering

We've grouped examples by functionality. Each group provides one or more
examples so you can start with the basics and then ramp up as you become more
comfortable.

We recommend reading the examples in the order shown below to allow them to
build upon one another. Each group provides it's own README with further
information on functionality and ordering.

| Functionality | Description | Count^
| --- | --- | ---:|
| [Hello World](hello_world) | The famous Hello World example application. | 1 |
| [Routing](routing) | Dispatching `Requests` to functionality provided by your application. | 4 |
| [Path](path) | Extracting data from the `Request` path ensuring type safety. | 1 |
| [Query String](query_string) | Extracting data from the `Request` query string whilst ensuring type safety. | 1 |
| [Cookies](cookies) | Working with Cookies. | 0 |
| [Headers](headers) | Working with HTTP Headers. | 1 |
| [Middleware](middleware) | Developing custom middleware for your application. | 0 |
| [Into Response](into_response) | Implementing the Gotham web framework's `IntoResponse` trait. | 1 |

^ Gotham web framework examples are under active development. 

## Contributing

We welcome example contributions from the community. To get started please see
the [example contribution template](example_contribution_template) README file
and starter crate.

## Help

You can get help for the Gotham web framework at:

* [The Gotham web framework website](https://gotham.rs)
* [Gotham web framework API documentation](https://docs.rs/gotham/)
* [Gitter chatroom](https://gitter.im/gotham-rs/gotham)
* [Twitter](https://twitter.com/gotham_rs)

## License

Licensed under your option of:

* [MIT License](../LICENSE-MIT)
* [Apache License, Version 2.0](../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../CONDUCT.md)
* [Contributing](../CONTRIBUTING.md)
