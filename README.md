<p align="center">
  <img src="https://gotham.rs/assets/brand/logo.svg" alt="The Gotham web framework" width="400" height="276">
</p>

A flexible web framework that promotes **stability, safety, security and speed**.

[![Join the chat at https://gitter.im/gotham-rs/gotham](https://badges.gitter.im/gotham-rs/gotham.svg)](https://gitter.im/gotham-rs/gotham?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)
[![GitHub actions](https://github.com/gotham-rs/gotham/workflows/Rust/badge.svg)](https://github.com/gotham-rs/gotham/actions?query=workflow%3ARust)
[![Dependency status](https://deps.rs/repo/github/gotham-rs/gotham/status.svg)](https://deps.rs/repo/github/gotham-rs/gotham)

## Additional feature of this fork:
* Support Borrowing Async Request Handlers(route: `to_async_borrowing`) by customizing error response (.await version), so you can use `?` shorthand.
* example usage:
```rust
pub async fn might_return_error_by_mapping_err_with_customized_response(
    state: &mut State,
) -> Result<impl IntoResponse, HandlerError> {
    let even = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() % 2;

    if even == 0 {
        // here, we just simulate an err.
        let _io_error = Err(std::io::Error::last_os_error())
            .map_err_with_customized_response(
                state,
                |_state| {
                    // an error occurs, but still sending **OK** to client
                    (StatusCode::SERVICE_UNAVAILABLE, mime::TEXT_PLAIN_UTF_8, "Error: Customized response by the last os error (Intentionally return 200 even error occurs) (even == 0)")
                },
            )?;
    }
    Ok(create_response(&state, StatusCode::OK, mime::TEXT_PLAIN_UTF_8, "even != 0"))
}
```
* see more examples at: `examples/handlers/simple_async_handlers_await_with_customized_error_response/src/main.rs`

This example shows how to customizing error response when different errors occur.What is this fork includes

## Features

1.  Stability focused. All releases target **stable**
    [Rust](https://www.rust-lang.org/en-US/). This will never
    change. To ensure future compatibility, we also run automated builds against
    Rust beta and nightly releases.
1.  Statically typed. The Gotham web framework is statically typed ensuring your
    application is **correctly expressed** at compile time.
1.  Async everything. By leveraging the [Tokio project](https://tokio.rs), all
    Gotham web framework types are async out of the box.  Our async story is
    further enhanced by [Hyper](https://hyper.rs), a fast server that provides
    an elegant layer over
    [stringly typed HTTP](http://wiki.c2.com/?StringlyTyped).
1.  Blazingly fast. Measure completed requests, including the 99th percentile,
    in **µs**.

## License

Licensed under your option of:

* [MIT License](LICENSE-MIT)
* [Apache License, Version 2.0](LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](CODE_OF_CONDUCT.md)
* [Contributing](CONTRIBUTING.md)

## Learning

The following resources are available to assist you learning the Gotham web
framework:

* [An extensive set of examples](examples)
* [API documentation](https://docs.rs/gotham/)
* [Gitter chatroom](https://gitter.im/gotham-rs/gotham)
* [Twitter](https://twitter.com/gotham_rs)
* [The Gotham web framework website](https://gotham.rs)

## Projects Using Gotham

* [Template for local GUIs with Seed and Gotham](https://gitlab.com/liketechnik/local-gui-seed-gotham)

## Alternatives

We hope you'll find the Gotham web framework is flexible enough to meet the
needs of any web application you might like to build. Please
[have a chat with us](https://gitter.im/gotham-rs/gotham) or
[create an issue](https://github.com/gotham-rs/gotham/issues) if you find this
isn't the case, perhaps there is something the Gotham web framework can offer
that will help you achieve your goals.

We do acknowledge that sometimes the choices we've made for the Gotham web
framework may not suit the needs of all projects. If that is the case for your
project there are alternative Rust web frameworks you might like to consider:

1. [Actix-Web](https://github.com/actix/actix-web)
1. [Conduit](https://github.com/conduit-rust/conduit)
1. [Nickel](https://github.com/nickel-org/nickel.rs)
1. [Rocket](https://github.com/SergioBenitez/Rocket)
1. [Rouille](https://github.com/tomaka/rouille)

Explore even more suggestions at [Are we web yet?](http://www.arewewebyet.org/).
