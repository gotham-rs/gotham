# Async Request Handlers (.await version)

The idea of async handlers has already been introduced by the post_handler example in
[Request Data](../request_data), which waits for the POST body asyncronously, and resolves
the response future once it has processed the body. The combinator-based version
of this example can be found at [Async Request Handlers](../simple_async_handlers).

This example has exactly the same behavior and API as the combinator-based version,
and it can be used as a reference when converting your code to use async/await.
It also leaves the versions of gotham, tokio and hyper the same, and uses the
compatibility helpers from the `futures` crate to convert things at the
interface boundaries.

## Running

From the `examples/handlers/async_handlers` directory:

```
Terminal 1:
   Compiling gotham_examples_handlers_simple_async_handlers v0.0.0 (file:///.../gotham/examples/handlers/simple_async_handlers)
    Finished dev [unoptimized + debuginfo] target(s) in 8.19 secs
     Running `.../gotham/target/debug/gotham_examples_handlers_simple_async_handlers`
Listening for requests at http://127.0.0.1:7878
sleep for 5 seconds once: starting
sleep for 5 seconds once: finished
sleep for one second 5 times: starting
sleep for one second 5 times: finished

Terminal 2:
$ curl 'http://127.0.0.1:7878/sleep?seconds=5'
slept for 5 seconds
$ curl 'http://127.0.0.1:7878/loop?seconds=5'
slept for 1 seconds
slept for 1 seconds
slept for 1 seconds
slept for 1 seconds
slept for 1 seconds
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Code of conduct](../../CODE_OF_CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
