# HTML Form Parsing

An example showing how to decode requests from an HTML form element with `Content-Type: application/x-www-form-urlencoded` data.

## Running

From the `examples/handlers/form_urlencoded` directory:

```
Terminal 1:
  $ cargo run
     Compiling handlers/form_urlencoded (file:///.../examples/handlers/form_urlencoded)
      Finished dev [unoptimized + debuginfo] target(s) in 4.26 secs
       Running `../handlers/form_urlencoded`
  Listening for requests at http://127.0.0.1:7878

Terminal 2:
  $ curl -d name=Bob -d address="123 Jersey Ave." -d message="Hello world!" http://127.0.0.1:7878/
  name: Bob
  address: 123 Jersey Ave.
  message: Hello world!
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Code of conduct](../../CODE_OF_CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
