# Async Request Handlers

The idea of async handlers has already been introduced by the post_handler example in
[Request Data](../request_data), which waits for the POST body asyncronously, and resolves
the response future once it has processed the body.

This is a contrived example, that calls itself recursively over http, to produce a string of
'z's of a requested length. This is not something that you would want to do in real life.
That said, the techniques used should be transferrable to any code that makes calls
to external services, and wants to do so without blocking other `Handler`s from running on the
same thread while it is waiting for a response.

## Running

From the `examples/handlers/async_handlers` directory:

```
Terminal 1:
$ cargo run
   Compiling handlers/async_handlers (file:///.../examples/handlers/async_handlers)
    Finished dev [unoptimized + debuginfo] target(s) in 4.26 secs
     Running `../handlers/async_handlers`
Listening for requests at http://127.0.0.1:7878
series length: 2 starting
series length: 1 starting
series length: 1 finished
series length: 1 starting
series length: 1 finished
series length: 2 finished
fold length: 2 starting
fold length: 1 starting
fold length: 1 finished
fold length: 1 starting
fold length: 1 finished
fold length: 2 finished
parallel length: 2 starting
parallel length: 1 starting
parallel length: 1 finished
parallel length: 1 starting
parallel length: 1 finished
parallel length: 2 finished

Terminal 2:
$ curl 'http://127.0.0.1:7878/series?length=2' && echo " = ok" || echo " = failed"
zz = ok
$ curl 'http://127.0.0.1:7878/fold?length=2' && echo " = ok" || echo " = failed"
zz = ok
$ curl 'http://127.0.0.1:7878/parallel?length=2' && echo " = ok" || echo " = failed"
zz = ok


```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Code of conduct](../../CODE_OF_CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
