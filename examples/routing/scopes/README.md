# Routing using Scopes

An example of the Gotham web framework `Router` that shows how to combine `Routes` under a
common root using scopes.

## Running

From the `examples/routing/scopes` directory:

```
Terminal 1:

   Compiling gotham_examples_routing_scopes v0.0.0 (file://.../examples/routing/scopes)
    Finished dev [unoptimized + debuginfo] target(s) in 2.57 secs
     Running `.../target/debug/gotham_examples_routing_scopes`
Listening for requests at http://127.0.0.1:7878

Terminal 2:

  $ curl -v http://127.0.0.1:7878/checkout/start
  *   Trying 127.0.0.1...
  * TCP_NODELAY set
  * Connected to 127.0.0.1 (127.0.0.1) port 7878 (#0)
  > GET /checkout/start HTTP/1.1
  > Host: 127.0.0.1:7878
  > User-Agent: curl/7.54.1
  > Accept: */*
  >
  < HTTP/1.1 200 OK
  < Content-Length: 5
  < Content-Type: text/plain
  < X-Request-ID: 4e90411d-aab5-4568-9f4d-0c4086a42c1a
  < X-Frame-Options: DENY
  < X-XSS-Protection: 1; mode=block
  < X-Content-Type-Options: nosniff
  < X-Runtime-Microseconds: 87
  < Date: Mon, 12 Feb 2018 10:54:12 GMT
  <
  * Connection #0 to host 127.0.0.1 left intact
  start%

```

## License

Licensed under your option of:

* [MIT License](../../../LICENSE-MIT)
* [Apache License, Version 2.0](../../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../../../CONDUCT.md)
* [Contributing](../../../CONTRIBUTING.md)
