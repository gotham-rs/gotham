# Path extraction introduction

An introduction to extracting request path segments, in a type safe way, with
the Gotham web framework

## Running

From the `examples/path/introduction` directory:

```
Terminal 1:
  $ cargo run
     Compiling gotham_examples_path_introduction v0.0.0 (file://.../gotham/examples/path/introduction)
      Finished dev [unoptimized + debuginfo] target(s) in 2.36 secs
       Running `.../gotham/target/debug/gotham_examples_path_introduction`
  Listening for requests at http://127.0.0.1:7878

Terminal 2:

  $ curl -vvv http://localhost:7878/products/t-shirt
  *   Trying 127.0.0.1...
  * TCP_NODELAY set
  * Connected to localhost (127.0.0.1) port 7878 (#0)
  > GET /products/t-shirt HTTP/1.1
  > Host: localhost:7878
  > User-Agent: curl/7.54.1
  > Accept: */*
  >
  < HTTP/1.1 200 OK
  < Content-Length: 16
  < Content-Type: text/plain
  < X-Request-ID: eb0457b0-a38e-4b8b-afb2-e2f3d9f1010e
  < X-Frame-Options: DENY
  < X-XSS-Protection: 1; mode=block
  < X-Content-Type-Options: nosniff
  < X-Runtime-Microseconds: 81
  < Date: Sat, 17 Feb 2018 05:21:31 GMT
  <
  * Connection #0 to host localhost left intact
  Product: t-shirt%
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../../CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
