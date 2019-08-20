# HTML Form Parsing

An example showing how to decode requests from an HTML form element with `Content-Type: multipart/form-data` data.

## Running

From the `examples/handlers/multipart` directory:

```
Terminal 1:
  $ cargo run
     Compiling handlers/multipart (file:///.../examples/handlers/multipart)
      Finished dev [unoptimized + debuginfo] target(s) in 4.26 secs
       Running `../handlers/multipart`
  Listening for requests at http://127.0.0.1:7878

Terminal 2:
  $curl -H "Content-Type: multipart/form-data; boundary=--abcdef1234" -d "--abcdef1234\r\nContent-Disposition: form-data; name=foo\r\n\r\nbar\r\n\--abcdef1234--\r\n\" http://127.0.0.1:7878
  bar
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Code of conduct](../../CODE_OF_CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
