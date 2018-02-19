> Delete blockquotes like this one from the final README, they are here to help
> example developers and do not form part of what should be provided in the
> final published example set.

> Ensure that you add your new example crate to the top level `Cargo.toml` file so that it is compiled and
> tested by travis.

# Example name

Shows how to ...

## Running

From the `examples/functionality/name` directory:

> Replace terminal output with real world expectations for running the example.

```
Terminal 1:
  $ cargo run
     Compiling gotham_examples_functionality_name v0.0.0 (file://.../gotham/examples/functionality/name)
      Finished dev [unoptimized + debuginfo] target(s) in 2.56 secs
       Running `.../gotham/target/debug/gotham_examples_functionality_name`
  Listening for requests at http://127.0.0.1:7878

Terminal 2:
  $ curl -v http://127.0.0.1:7878
  ...
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../../CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
