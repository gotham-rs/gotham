# Examples

A collection of crates that provide examples for developing web applications
with the Gotham web framework.

All crates include test cases demonstrating correct behaviour and serve as an 
example of how to test your applications.

## Rust

These examples assume a familiarity with the Rust programming language. If
you're new to Rust itself the following resources will assist you to get
started:

1. [The Rust book](https://doc.rust-lang.org/book/second-edition/).
1. [Programming Rust](http://shop.oreilly.com/product/0636920040385.do)
1. [Exercisms for Rust](http://exercism.io/languages/rust/about)
1. [The Rust users community](https://users.rust-lang.org)
1. [Rust subreddit](https://reddit.com/r/rust)

## Help

Our welcoming community is available to help:

* [The Gotham web framework website](https://gotham.rs)
* [Gotham web framework API documentation](https://docs.rs/gotham/)
* [Gitter chatroom](https://gitter.im/gotham-rs/gotham)
* [Twitter](https://twitter.com/gotham_rs)

## Ordering

We've grouped our examples by areas of major functionality. Each area provides
one or more examples so you can start with the basics and then ramp up as you 
become more comfortable working with the Gotham web framework. 

We recommend reviewing in the order shown below as both a reasonable
way to introduce yourself to the Gotham web framework and to allow examples to 
build upon one another. Each group provides it's own README with information 
on the functionality it exposes and further ordering suggestions.

1. [Hello World](hello_world) - The famous Hello World example application.
1. [Routing](routing) - Dispatching `Requests` to functionality provided by your application.
1. [Path](path) - Extracting data from the `Request` path.
1. [Query String](query_string) - Extracting data from the `Request` query string.
1. [Cookies](cookies) - Working with Cookies.
1. [Headers](headers) - Working with HTTP Headers.
1. [Into Response](into_response) - Leveraging the Gotham `IntoResponse` trait.

## License

Licensed under your option of:

* [MIT License](../LICENSE-MIT)
* [Apache License, Version 2.0](../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../../CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
