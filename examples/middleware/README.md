# Middleware and Pipeline Examples

A collection of crates that provide examples of Gotham web framework
`Middleware` and `Pipelines`.

`Middleware` has the ability to interact with both the `Request` and `Response`.
The Gotham web framework combines a series of `Middleware` through `Pipelines`
allowing considerable flexibility for logic which is necessary across `Routes`
such as authentication and access control.

## Ordering

We recommend reviewing our middleware and pipline examples in the order shown 
below:

1. [Introduction](introduction) - Introduces the `Middleware` and `Pipeline` concepts.

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

* [Conduct](../../CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
