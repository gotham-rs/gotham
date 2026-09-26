# Routing on the Accept Header

An example of the Gotham web framework `Router` that shows how to dispatch
requests for the same path to different handlers, depending on the media types
listed in the request's `Accept` header.

## Running

From the `examples/routing/accept_header` directory:

```
Terminal 1:
$ cargo run
   Compiling gotham_examples_routing_accept_header v0.0.0 (file://.../examples/routing/accept_header)
    Finished dev [unoptimized + debuginfo] target(s)
     Running `.../target/debug/gotham_examples_routing_accept_header`
Listening for requests at http://127.0.0.1:7878

Terminal 2:
$ curl -H 'Accept: application/json' http://127.0.0.1:7878/greeting
{"greeting":"Hello World!"}

$ curl -H 'Accept: text/plain' http://127.0.0.1:7878/greeting
Hello World!

$ curl -H 'Accept: application/xml, text/*;q=0.9' http://127.0.0.1:7878/greeting
Hello World!

$ curl -i -H 'Accept: image/png' http://127.0.0.1:7878/greeting
HTTP/1.1 406 Not Acceptable
...
```

## License

Licensed under your option of:

* [MIT License](../../../LICENSE-MIT)
* [Apache License, Version 2.0](../../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Code of conduct](../../../CODE_OF_CONDUCT.md)
* [Contributing](../../../CONTRIBUTING.md)
