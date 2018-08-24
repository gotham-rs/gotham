# Stateful Handlers

An example of using stateful handlers with the Gotahm web framework.

If your state is tied to the request being made, or the user of the application,
it may make more sense to use [Middleware](../../middleware) (and perhaps in
particular the [Session middleware](../../sessions)).

## Running

From the `examples/handlers/stateful` directory:

```
Terminal 1:
  $ cargo run
     Compiling gotham_examples_handlers/stateful v0.0.0 (file://.../gotham/examples/handlers/stateful)
      Finished dev [unoptimized + debuginfo] target(s) in 2.49 secs
       Running `.../gotham/target/debug/gotham_examples_handlers/stateful`
  Listening for requests at http://127.0.0.1:7878

Terminal 2:
  $ curl http://127.0.0.1:7878
This server has been up for 1 second(s). This is visit number 1.
  $ curl http://127.0.0.1:7878
This server has been up for 5 second(s). This is visit number 2.

```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Code of conduct](../../CODE_OF_CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
