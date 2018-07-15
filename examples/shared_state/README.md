# Shared State

A simple introduction to shared state across Gotham handlers.

## Running

From the `examples/shared_state` directory:

```
Terminal 1:
$ cargo run
   Compiling gotham_examples_shared_state (file:///.../examples/shared_state)
    Finished dev [unoptimized + debuginfo] target(s) in 4.26 secs
     Running `../gotham_examples_shared_state`
  Listening for requests at http://127.0.0.1:7878

Terminal 2:
$ curl -v http://127.0.0.1:7878/
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to 127.0.0.1 (127.0.0.1) port 7878 (#0)
> GET / HTTP/1.1
> Host: 127.0.0.1:7878
> User-Agent: curl/7.54.0
> Accept: */*
>
< HTTP/1.1 200 OK
< Content-Type: text/plain
< Content-Length: 22
< X-Request-ID: b3fd0952-31f9-4872-8bbd-7fd6c07a0589
< X-Frame-Options: DENY
< X-XSS-Protection: 1; mode=block
< X-Content-Type-Options: nosniff
< X-Runtime-Microseconds: 165
< Date: Sun, 15 Jul 2018 00:39:41 GMT
<
* Connection #0 to host 127.0.0.1 left intact
Hello from request #1!

$ curl -v http://127.0.0.1:7878/
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to 127.0.0.1 (127.0.0.1) port 7878 (#0)
> GET / HTTP/1.1
> Host: 127.0.0.1:7878
> User-Agent: curl/7.54.0
> Accept: */*
>
< HTTP/1.1 200 OK
< Content-Type: text/plain
< Content-Length: 22
< X-Request-ID: 546d24e4-d7ad-4d69-be68-4e55cc4b9b96
< X-Frame-Options: DENY
< X-XSS-Protection: 1; mode=block
< X-Content-Type-Options: nosniff
< X-Runtime-Microseconds: 118
< Date: Sun, 15 Jul 2018 00:39:44 GMT
<
* Connection #0 to host 127.0.0.1 left intact
Hello from request #2!
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Code of conduct](../../CODE_OF_CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
