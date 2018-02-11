# Routing using HTTP Verbs

An example of the Gotham web framework `Router` that shows how to route `Requests`
to handlers based on HTTP verbs.

## Running

From the `examples/routing/http_verbs` directory:

```
Terminal 1:

$ cargo run                                                                                                                                                                    101 ↵
   Compiling gotham_examples_routing_verbs v0.0.0 (file:///.../examples/routing/http_verbs)
    Finished dev [unoptimized + debuginfo] target(s) in 2.59 secs
     Running `.../target/debug/gotham_examples_routing_http_verbs`
Listening for requests at http://127.0.0.1:7878


Terminal 2:

$ curl -v http://127.0.0.1:7878/products
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to 127.0.0.1 (127.0.0.1) port 7878 (#0)
> GET /products HTTP/1.1
> Host: 127.0.0.1:7878
> User-Agent: curl/7.54.1
> Accept: */*
>
< HTTP/1.1 200 OK
< Content-Length: 5
< Content-Type: text/plain
< X-Request-ID: f25a879f-64d0-4415-9b33-3145137b0f01
< X-Frame-Options: DENY
< X-XSS-Protection: 1; mode=block
< X-Content-Type-Options: nosniff
< X-Runtime-Microseconds: 147
< Date: Sun, 11 Feb 2018 22:46:00 GMT
<
* Connection #0 to host 127.0.0.1 left intact
index%

```

## License

Licensed under your option of:

* [MIT License](../../../LICENSE-MIT)
* [Apache License, Version 2.0](../../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../../../CONDUCT.md)
* [Contributing](../../../CONTRIBUTING.md)
