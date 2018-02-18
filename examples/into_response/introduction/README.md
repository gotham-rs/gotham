# IntoResponse introduction

An introduction to the Gotham web framework's `IntoResponse` trait.

## Running

From the `examples/into_response/introduction` directory:

```
Terminal 1:
  $ cargo run
     Compiling gotham_examples_into_response_introduction v0.0.0 (file://.../gotham/examples/into_response/introduction)
       Finished dev [unoptimized + debuginfo] target(s) in 2.35 secs
       Running `.../gotham/target/debug/gotham_examples_into_response_introduction`
  Listening for requests at http://127.0.0.1:7878

Terminal 2:
  $ curl -v http://localhost:7878/products/t-shirt
  *   Trying 127.0.0.1...
  * TCP_NODELAY set
  * Connected to localhost (127.0.0.1) port 7878 (#0)
  > GET /products/t-shirt HTTP/1.1
  > Host: localhost:7878
  > User-Agent: curl/7.54.1
  > Accept: */*
  >
  < HTTP/1.1 200 OK
  < Content-Length: 18
  < Content-Type: application/json
  < X-Request-ID: caa738c3-7467-42d0-b950-2c7b17643b22
  < X-Frame-Options: DENY
  < X-XSS-Protection: 1; mode=block
  < X-Content-Type-Options: nosniff
  < X-Runtime-Microseconds: 140
  < Date: Sun, 18 Feb 2018 00:03:17 GMT
  <
  * Connection #0 to host localhost left intact
  {"name":"t-shirt"}%
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../../CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
