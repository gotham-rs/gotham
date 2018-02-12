# Routing using Associations

An example of the Gotham web framework `Router` that shows how to associate
multiple handlers to a single path.

## Running

From the `examples/routing/associations` directory:

```
Terminal 1:

$ cargo run
   Compiling gotham_examples_routing_associations v0.0.0 (file://.../gotham/examples/routing/associations)
    Finished dev [unoptimized + debuginfo] target(s) in 2.48 secs
     Running `.../gotham/target/debug/gotham_examples_routing_associations`
Listening for requests at http://127.0.0.1:7878

Terminal 2:

$ curl -v -X PUT http://127.0.0.1:7878/checkout/address                                                                                                                          2 ↵
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to 127.0.0.1 (127.0.0.1) port 7878 (#0)
> PUT /checkout/address HTTP/1.1
> Host: 127.0.0.1:7878
> User-Agent: curl/7.54.1
> Accept: */*
>
< HTTP/1.1 200 OK
< Content-Length: 6
< Content-Type: text/plain
< X-Request-ID: 2d1f738c-c091-4f34-990d-6ae7f82fb6d0
< X-Frame-Options: DENY
< X-XSS-Protection: 1; mode=block
< X-Content-Type-Options: nosniff
< X-Runtime-Microseconds: 104
< Date: Mon, 12 Feb 2018 23:41:57 GMT
<
* Connection #0 to host 127.0.0.1 left intact
update%

```

## License

Licensed under your option of:

* [MIT License](../../../LICENSE-MIT)
* [Apache License, Version 2.0](../../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../../../CONDUCT.md)
* [Contributing](../../../CONTRIBUTING.md)
