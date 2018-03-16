# Async Request Handlers

The idea of async handlers has already been introduced by the post_handler example in
[Request Data](../request_data), which waits for the POST body asyncronously, and resolves
the response future once it has processed the body.

This example contains a pair of endpoints that sleep for a number of seconds,
in different ways. Note that we never call `std::thread::sleep` in this example,
so none of our request handlers block the thread from handling other requests
in parallel. Instead, in each case, we return a future, which will resolve when
the requested time has elapsed, and cause Gotham to respond to the http request.

The approach of using futures to track the status of long-running operations
is significantly lower overhead than that of spawning a new thread per request.
In our case, the long-running operations are sleeps, but in
[Async Handlers](../async_handlers), the long-running operations are http
requests.

You will often find that most of the time that a web server spends dealing
with a web request, it is waiting on another service (e.g. a database, or an
external api). If you can track these operations using futures, you should end
up with a very lightweight and performant web server. On the other hand, if you
find yourself doing lots of CPU/memory intensive operations on the web server,
then futures are probably not going to help your performance, and you might be
better off spawning a new thread per request.

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
