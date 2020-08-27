# Handler Examples

A collection of crates that provide examples of handlers, the primary building 
block of a Gotham web framework application.

A `Handler` is an asynchronous function, taking a `State` value which 
represents the request and related runtime state. It completes by resolving to 
a HTTP response.

Developers working with the Gotham web framework create handlers and via the
`Router` map them to individual routes. The `Router` then invokes the
appropriate `Handler` to process incoming requests.

## Ordering

We recommend reviewing our handler examples in the order shown below:

1. [Request Data](request_data) - Accessing common request information
1. [Stateful Handlers](stateful) - Keeping state in a handler
1. [Simple Async Handlers](simple_async_handlers) - Async Request Handlers 101
1. [Simple Async Handlers (.await version)](simple_async_handlers_await) - Request Handlers that use async/.await
1. [Async Handlers](async_handlers) - More complicated async request handlers

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

* [Code of conduct](../../CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
