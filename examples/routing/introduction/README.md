# Introductory Router Example

An introduction to fundamental `Router` and `Router Builder` concepts to create a routing tree.

## Running

From the `examples/introduction` directory:

```
Terminal 1:

$ cargo run
   Compiling gotham_examples_routing_introduction v0.0.0 (file://.../examples/routing/introduction)
    Finished dev [unoptimized + debuginfo] target(s) in 2.42 secs
     Running `.../target/debug/gotham_examples_routing_introduction`
Listening for requests at http://127.0.0.1:7878

Terminal 2:
$ curl -v http://127.0.0.1:7878/
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to 127.0.0.1 (127.0.0.1) port 7878 (#0)
> GET / HTTP/1.1
> Host: 127.0.0.1:7878
> User-Agent: curl/7.54.1
> Accept: */*
>
< HTTP/1.1 200 OK
< Content-Length: 13
< Content-Type: text/plain
< X-Request-ID: e4c3fb68-9f70-43ee-9fd2-1bb0ec7a7548
< X-Frame-Options: DENY
< X-XSS-Protection: 1; mode=block
< X-Content-Type-Options: nosniff
< X-Runtime-Microseconds: 123
< Date: Sun, 11 Feb 2018 22:55:26 GMT
<
* Connection #0 to host 127.0.0.1 left intact
Hello Router!%

```

## License

Licensed under your option of:

* [MIT License](../../../LICENSE-MIT)
* [Apache License, Version 2.0](../../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../../../CONDUCT.md)
* [Contributing](../../../CONTRIBUTING.md)
