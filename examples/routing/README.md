# Routing Examples

A collection of crates that provide examples of the Gotham web framework Router.

The `Router` dispatches `Requests` to a linked `Route` and handles error
responses when a valid `Route` is missing or an internal error state occurs.

## Ordering

We recommend reviewing our routing examples in the order shown below:

1. [Introduction](introduction) - Introduces fundamental router and router builder concepts.
1. [HTTP Verbs](http_verbs) - Shows how to route requests to handlers based on HTTP verbs.
1. [Scopes](scopes) - Combining routes under a common, nestable, root.
1. [Associations](associations) - Associate multiple handlers to a single path.

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
