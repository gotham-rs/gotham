# Query string extraction introduction

An introduction to extracting query string name/value pairs, in a type safe way, with the Gotham web framework.

## Running

From the `examples/query_string/introduction` directory:

```
Terminal 1:
  $ cargo run                                                                                                                                                                                                                     130 ↵
     Compiling gotham_examples_query_string_introduction v0.0.0 (file://.../gotham/examples/query_string/introduction)
      Finished dev [unoptimized + debuginfo] target(s) in 2.49 secs
       Running `.../gotham/target/debug/gotham_examples_query_string_introduction`
  Listening for requests at http://127.0.0.1:7878

Terminal 2:
  $ curl -v http://localhost:7878/products\?name\=t-shirt
  *   Trying 127.0.0.1...
  * TCP_NODELAY set
  * Connected to localhost (127.0.0.1) port 7878 (#0)
  > GET /products?name=t-shirt HTTP/1.1
  > Host: localhost:7878
  > User-Agent: curl/7.54.1
  > Accept: */*
  >
  < HTTP/1.1 200 OK
  < Content-Length: 18
  < Content-Type: application/json
  < X-Request-ID: 561c8c45-9945-4b01-ba8d-066a44761686
  < X-Frame-Options: DENY
  < X-XSS-Protection: 1; mode=block
  < X-Content-Type-Options: nosniff
  < X-Runtime-Microseconds: 170
  < Date: Sat, 17 Feb 2018 10:33:05 GMT
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
